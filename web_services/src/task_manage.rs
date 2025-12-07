use actix_web::web;
use common::TaskFilter;
use common::api::ApiResponse;
use common::po::{ItemResult, QueryPage, TaskItem};
use common::utils::date_utils::get_today_weekday;
use infra::{
    list_all_scheduled_tasks_by_page, upsert_ani_info, upsert_news_info, upsert_video_info,
};
use serde_json::json;
use service::commands::{CmdFn, build_cmd_map};
use service::process_news_info_to_item::process_news;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use timer_tasker::scheduler::Scheduler;
use timer_tasker::task::TaskMeta;
use timer_tasker::task::TaskResult;
use timer_tasker::task::build_tasks_from_meta;
use tokio::sync::{RwLock, mpsc};
use tracing::{error, info, warn};

// 全局单例
static GLOBAL_TASK_MANAGER: OnceLock<Arc<TaskManager>> = OnceLock::new();

// 全局定时任务管理器
pub struct TaskManager {
    db_pool: Arc<PgPool>,
    // 当前运行的调度器（用于重启）
    current_scheduler: Arc<RwLock<Option<Arc<Scheduler>>>>,
    // 命令映射表
    cmd_map: HashMap<String, CmdFn>, // CmdFn 是你的命令函数类型
}

// 初始化并启动 TaskManager
pub async fn initialize_task_manager(db_pool: PgPool) -> anyhow::Result<()> {
    let task_manager = Arc::new(TaskManager::new(db_pool.clone()));

    // 设置全局实例
    GLOBAL_TASK_MANAGER
        .set(task_manager.clone())
        .map_err(|_| anyhow::anyhow!("TaskManager 已经初始化"))?;

    // 启动定时任务
    start_async_timer_task_with_manager(task_manager).await;

    Ok(())
}

// 获取全局 TaskManager 实例
pub fn get_global_task_manager() -> Option<Arc<TaskManager>> {
    GLOBAL_TASK_MANAGER.get().cloned()
}

// 修改后的启动函数，接收 TaskManager 实例
async fn start_async_timer_task_with_manager(task_manager: Arc<TaskManager>) {
    if let Err(e) = task_manager.start_or_restart_tasks().await {
        error!("定时任务启动失败: {e:?}");
    }
}

impl TaskManager {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool: Arc::new(db_pool),
            current_scheduler: Arc::new(RwLock::new(None)),
            cmd_map: build_cmd_map(),
        }
    }

    // 启动或重启定时任务系统
    pub async fn start_or_restart_tasks(&self) -> anyhow::Result<()> {
        // 1. 停止现有的调度器
        self.stop_current_scheduler().await;

        // 2. 重新加载配置
        let task_conf = self.load_task_config().await?;

        // 3. 构建任务
        let tasks = build_tasks_from_meta(&task_conf, &self.cmd_map, self.db_pool.clone());

        // 4. 创建新调度器
        let scheduler = Arc::new(Scheduler::new(tasks, None));

        // 5. 保存新调度器引用
        {
            let mut current = self.current_scheduler.write().await;
            *current = Some(scheduler.clone());
        }

        // 6. 启动结果接收器和新调度器
        self.start_scheduler_with_channel(scheduler).await;

        Ok(())
    }

    // 加载任务配置
    pub async fn load_task_config(&self) -> anyhow::Result<Vec<TaskMeta>> {
        let query = create_empty_query();
        let timer_tasker = list_all_scheduled_tasks_by_page(query, &self.db_pool).await?;

        let task_metas = timer_tasker
            .items
            .iter()
            .map(|task| {
                let cmd = task.params["cmd"].as_str().unwrap_or("").to_string();
                let arg = task.params["arg"].as_str().unwrap_or("").to_string();

                TaskMeta {
                    name: task.name.clone(),
                    cmd,
                    arg,
                    cron_expr: task.cron.clone(),
                    retry_times: task.retry_times,
                }
            })
            .collect::<Vec<TaskMeta>>();

        Ok(task_metas)
    }

    // 强制刷新配置
    pub async fn refresh_config(&self) -> anyhow::Result<()> {
        // 重新启动任务
        self.start_or_restart_tasks().await
    }

    // 停止当前调度器
    async fn stop_current_scheduler(&self) {
        let mut current = self.current_scheduler.write().await;
        // 安全地停止调度器
        if let Some(scheduler) = current.take() {
            scheduler.stop();
            info!("已停止当前的定时任务调度器");
        }
    }

    // 启动调度器（包含通道创建）
    async fn start_scheduler_with_channel(&self, scheduler: Arc<Scheduler>) {
        let connect_pool = Arc::clone(&self.db_pool);

        // 创建 mpsc channel 用于接收 TaskResult
        let (tx, mut rx) = mpsc::channel::<TaskResult>(128);

        // 启动结果接收器
        tokio::spawn({
            let connect_pool = Arc::clone(&connect_pool);
            async move {
                while let Some(res) = rx.recv().await {
                    if let Some(item_result) = res.result {
                        let pool_clone = Arc::clone(&connect_pool);
                        tokio::spawn(async move {
                            if let Err(e) = run_task_service(item_result, pool_clone).await {
                                warn!("task {:?} 保存失败", e);
                            }
                        });
                    }
                }
            }
        });

        // 启动调度器
        tokio::spawn({
            async move {
                scheduler.run(tx).await;
            }
        });
    }
}

pub async fn run_task_service(
    item_result: ItemResult,
    pool: Arc<PgPool>,
) -> anyhow::Result<ApiResponse, String> {
    // 启动定时任务服务
    let weekday = get_today_weekday().name_cn.to_string();

    let items = match item_result.get(&weekday) {
        Some(v) if !v.is_empty() => v,
        Some(_) => return Ok(ApiResponse::ok(json!({ "message": "没有可插入的数据" }))),
        None => return Ok(ApiResponse::err("获取更新数据失败")),
    };

    for item in items {
        if let Err(e) = handle_item(item, &pool).await {
            return Ok(ApiResponse::err(format!("插入或更新失败：{e}")));
        }
    }
    Ok(ApiResponse::ok(json!({ "message": "upsert success" })))
}

async fn handle_item(item: &TaskItem, pool: &PgPool) -> anyhow::Result<()> {
    match item {
        TaskItem::Ani(ani) => {
            upsert_ani_info(ani, pool).await?;
        }
        TaskItem::Video(video) => {
            upsert_video_info(video, pool).await?;
        }
        TaskItem::News(news) => {
            upsert_news_info(news, pool).await?;
        }
        TaskItem::Health(health) => {
            info!("健康检测结果: {} => {}", health.url, health.result);
        }
        TaskItem::ExtractNewsItem(new_item) => {
            process_news(new_item, pool).await?;
        }
        TaskItem::ExtractNewsNewsKeywords(res) => {
            info!("新闻keywords提取结果: {} => {}", res.url, res.result);
        }
    }
    Ok(())
}

/// 创建空的分页查询条件
fn create_empty_query() -> web::Query<QueryPage<TaskFilter>> {
    let filter = TaskFilter {
        name: None,
        arg: None,
        cmd: None,
    };

    let query_page = QueryPage {
        page: None,
        filter: Some(filter),
        page_size: None,
    };

    web::Query(query_page)
}
