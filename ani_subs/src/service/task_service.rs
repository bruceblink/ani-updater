use crate::dao::{
    list_all_scheduled_tasks_by_page, upsert_ani_info, upsert_news_info, upsert_video_info,
};
use crate::domain::po::QueryPage;
use crate::routes::TaskFilter;
use actix_web::web;
use chrono::{DateTime, Utc};
use common::api::ApiResponse;
use common::api::{ItemResult, TaskItem};
use common::utils::date_utils::get_today_weekday;
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use timer_tasker::commands::build_cmd_map;
use timer_tasker::scheduler::Scheduler;
use timer_tasker::task::TaskResult;
use timer_tasker::task::build_tasks_from_meta;
use timer_tasker::task::{CmdFn, TaskMeta};
use tokio::sync::{RwLock, mpsc};
use tracing::{error, info, warn};

// 添加配置缓存结构
#[derive(Debug, Clone)]
struct CachedTaskConfig {
    task_metas: Vec<TaskMeta>,
    last_updated: DateTime<Utc>,
    #[allow(dead_code)]
    version: u64,
}

// 全局任务管理器
pub struct TaskManager {
    db_pool: Arc<PgPool>,
    // 配置缓存
    config_cache: Arc<RwLock<Option<CachedTaskConfig>>>,
    // 当前运行的调度器（用于重启）
    current_scheduler: Arc<RwLock<Option<Arc<Scheduler>>>>,
    // 命令映射表
    cmd_map: HashMap<String, CmdFn>, // 假设 CmdFn 是你的命令函数类型
}

impl TaskManager {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool: Arc::new(db_pool),
            config_cache: Arc::new(RwLock::new(None)),
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
        let tasks = build_tasks_from_meta(&task_conf, &self.cmd_map);

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

    // 加载任务配置（带缓存）
    pub async fn load_task_config(&self) -> anyhow::Result<Vec<TaskMeta>> {
        // 检查缓存
        {
            let cache = self.config_cache.read().await;
            if let Some(cached) = &*cache {
                // 5分钟缓存
                let cache_age = Utc::now() - cached.last_updated;
                if cache_age < chrono::Duration::minutes(5) {
                    return Ok(cached.task_metas.clone());
                }
            }
        }

        // 缓存未命中或过期，从数据库加载
        let task_metas = self.load_config_from_db().await?;

        // 更新缓存
        let mut cache = self.config_cache.write().await;
        *cache = Some(CachedTaskConfig {
            task_metas: task_metas.clone(),
            last_updated: Utc::now(),
            version: 1, // 可以基于时间戳或版本号
        });

        Ok(task_metas)
    }

    // 从数据库加载配置
    async fn load_config_from_db(&self) -> anyhow::Result<Vec<TaskMeta>> {
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
        // 清空缓存
        {
            let mut cache = self.config_cache.write().await;
            *cache = None;
        }

        // 重新启动任务
        self.start_or_restart_tasks().await
    }

    // 停止当前调度器
    async fn stop_current_scheduler(&self) {
        let mut current = self.current_scheduler.write().await;
        *current = None;
        // 注意：这里需要根据你的 Scheduler 实现来正确停止
        // 如果 Scheduler 有 stop/shutdown 方法，应该在这里调用
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
                    if let Some(ani_item_result) = res.result {
                        let pool_clone = Arc::clone(&connect_pool);
                        tokio::spawn(async move {
                            if let Err(e) = run_task_service(ani_item_result, pool_clone).await {
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

    pub async fn check_config_updated(&self) -> anyhow::Result<bool> {
        // 这里可以实现更智能的检查逻辑
        // 例如：查询数据库中的最大更新时间戳

        let current_cache = self.config_cache.read().await;
        if let Some(cached) = &*current_cache {
            // 简单的基于时间的检查（5分钟缓存）
            let cache_age = Utc::now() - cached.last_updated;
            if cache_age > chrono::Duration::minutes(5) {
                return Ok(true);
            }
        } else {
            // 没有缓存，需要加载
            return Ok(true);
        }

        Ok(false)
    }
}

// 配置监听器
async fn start_config_watcher(task_manager: Arc<TaskManager>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(600)); // 10分钟检查一次

        loop {
            interval.tick().await;

            // 检查配置是否有更新（基于版本号或时间戳）
            if let Ok(should_reload) = task_manager.check_config_updated().await
                && should_reload
            {
                info!("检测到配置变更，重新加载定时任务...");
                if let Err(e) = task_manager.refresh_config().await {
                    error!("配置重载失败: {}", e);
                }
            }
        }
    });
}

/// 启动异步定时任务
pub async fn start_async_timer_task(connect_pool: PgPool) {
    // 创建任务管理器
    let task_manager = Arc::new(TaskManager::new(connect_pool));

    // 首次启动任务
    if let Err(e) = task_manager.start_or_restart_tasks().await {
        error!("定时任务启动失败: {e:?}");
        return;
    }
    // 启动配置监听
    start_config_watcher(task_manager.clone()).await;
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

/// 初始化定时任务配置
pub async fn init_scheduled_tasks_config(db_pool: &PgPool) -> Vec<TaskMeta> {
    // 创建空的分页查询条件，用于使用分页查询的函数查询所有数据
    let query = create_empty_query();
    match list_all_scheduled_tasks_by_page(query, db_pool).await {
        Ok(timer_tasker) => {
            timer_tasker
                .items
                .iter()
                .map(|task| {
                    // 安全地提取字符串值
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
                .collect::<Vec<TaskMeta>>()
        }
        Err(e) => {
            error!("定时任务初始化配置错误: {e:?}");
            Vec::new()
        }
    }
}
