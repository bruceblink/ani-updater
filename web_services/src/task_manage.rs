use actix_web::web;
use common::TaskFilter;
use common::po::{ItemResult, QueryPage, TaskItem};
use common::utils::date_utils::get_today_weekday;
use infra::{list_all_scheduled_tasks_by_page, update_scheduled_task_runtime, upsert_news_info};
use service::timer_task_command::{CmdFn, build_cmd_map};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use timer_tasker::scheduler::Scheduler;
use timer_tasker::task::TaskMeta;
use timer_tasker::task::TaskResult;
use timer_tasker::task::build_tasks_from_meta;
use tokio::sync::{Mutex, RwLock, Semaphore, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{Duration, timeout};
use tracing::{error, info, warn};

static GLOBAL_TASK_MANAGER: OnceLock<Arc<TaskManager>> = OnceLock::new();

struct SchedulerRuntime {
    scheduler: Arc<Scheduler>,
    scheduler_handle: JoinHandle<()>,
    result_handle: JoinHandle<()>,
}

pub struct TaskManager {
    db_pool: Arc<PgPool>,
    current_runtime: Arc<RwLock<Option<SchedulerRuntime>>>,
    restart_lock: Arc<Mutex<()>>,
    cmd_map: HashMap<String, CmdFn>,
}

pub async fn initialize_task_manager(db_pool: PgPool) -> anyhow::Result<()> {
    let task_manager = Arc::new(TaskManager::new(db_pool.clone()));

    GLOBAL_TASK_MANAGER
        .set(task_manager.clone())
        .map_err(|_| anyhow::anyhow!("TaskManager 已经初始化"))?;

    start_async_timer_task_with_manager(task_manager).await;

    Ok(())
}

pub fn get_global_task_manager() -> Option<Arc<TaskManager>> {
    GLOBAL_TASK_MANAGER.get().cloned()
}

async fn start_async_timer_task_with_manager(task_manager: Arc<TaskManager>) {
    if let Err(e) = task_manager.start_or_restart_tasks().await {
        error!("定时任务启动失败: {e:?}");
    }
}

impl TaskManager {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool: Arc::new(db_pool),
            current_runtime: Arc::new(RwLock::new(None)),
            restart_lock: Arc::new(Mutex::new(())),
            cmd_map: build_cmd_map(),
        }
    }

    pub async fn start_or_restart_tasks(&self) -> anyhow::Result<()> {
        let _restart_guard = self.restart_lock.lock().await;
        self.stop_current_scheduler().await;

        let task_conf = self.load_task_config().await?;
        let tasks = build_tasks_from_meta(&task_conf, &self.cmd_map);
        let scheduler = Arc::new(Scheduler::new(tasks, None));
        let (scheduler_handle, result_handle) =
            self.start_scheduler_with_channel(scheduler.clone());

        {
            let mut current = self.current_runtime.write().await;
            *current = Some(SchedulerRuntime {
                scheduler,
                scheduler_handle,
                result_handle,
            });
        }

        Ok(())
    }

    pub async fn load_task_config(&self) -> anyhow::Result<Vec<TaskMeta>> {
        let query = create_empty_query();
        let timer_tasker = list_all_scheduled_tasks_by_page(query, &self.db_pool).await?;

        let task_metas = timer_tasker
            .items
            .iter()
            .filter_map(|task| {
                let cmd = task.params["cmd"].as_str().unwrap_or("").to_string();
                if cmd.is_empty() {
                    warn!("任务 [{}] 缺少 params.cmd 字段，跳过加载", task.name);
                    return None;
                }
                let url = task.params["url"].as_str().unwrap_or("").to_string();
                let arg = task.params["arg"].as_str().unwrap_or("").to_string();
                Some(TaskMeta {
                    name: task.name.clone(),
                    cmd,
                    url,
                    arg,
                    cron_expr: task.cron.clone(),
                    retry_times: task.retry_times,
                })
            })
            .collect::<Vec<TaskMeta>>();

        Ok(task_metas)
    }

    pub async fn refresh_config(&self) -> anyhow::Result<()> {
        self.start_or_restart_tasks().await
    }

    async fn stop_current_scheduler(&self) {
        let runtime = {
            let mut current = self.current_runtime.write().await;
            current.take()
        };

        let Some(runtime) = runtime else {
            return;
        };

        runtime.scheduler.stop();
        info!("已停止当前的定时任务调度器");

        Self::await_shutdown("scheduler", runtime.scheduler_handle).await;
        Self::await_shutdown("result-processor", runtime.result_handle).await;
    }

    async fn await_shutdown(name: &str, handle: JoinHandle<()>) {
        match timeout(Duration::from_secs(2), handle).await {
            Ok(Ok(())) => info!(component = name, "任务已退出"),
            Ok(Err(e)) => warn!(component = name, error = %e, "任务退出异常"),
            Err(_) => warn!(component = name, "等待任务退出超时"),
        }
    }

    fn start_scheduler_with_channel(
        &self,
        scheduler: Arc<Scheduler>,
    ) -> (JoinHandle<()>, JoinHandle<()>) {
        let connect_pool = Arc::clone(&self.db_pool);

        let (tx, mut rx) = mpsc::channel::<TaskResult>(128);
        let worker_parallelism = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .max(4);
        let result_worker_limit = Arc::new(Semaphore::new(worker_parallelism));

        let result_handle = tokio::spawn({
            let connect_pool = Arc::clone(&connect_pool);
            let result_worker_limit = Arc::clone(&result_worker_limit);
            async move {
                while let Some(res) = rx.recv().await {
                    if let Err(e) = update_scheduled_task_runtime(
                        &res.name,
                        res.last_run,
                        res.next_run,
                        &res.last_status,
                        &connect_pool,
                    )
                    .await
                    {
                        warn!("更新任务 [{}] 运行态失败: {e:?}", res.name);
                    }

                    if let Some(item_result) = res.result {
                        let pool_clone = Arc::clone(&connect_pool);
                        let task_name = res.name.clone();
                        match result_worker_limit.clone().acquire_owned().await {
                            Ok(permit) => {
                                tokio::spawn(async move {
                                    let _permit = permit;
                                    if let Err(e) = run_task_service(item_result, pool_clone).await
                                    {
                                        warn!("任务 [{}] 保存结果失败: {:?}", task_name, e);
                                    }
                                });
                            }
                            Err(e) => {
                                warn!("任务 [{}] 无法获取结果处理许可: {}", task_name, e);
                            }
                        }
                    }
                }
            }
        });

        let scheduler_handle = tokio::spawn(async move {
            scheduler.run(tx).await;
        });

        (scheduler_handle, result_handle)
    }
}

pub async fn run_task_service(item_result: ItemResult, pool: Arc<PgPool>) -> anyhow::Result<()> {
    let weekday = get_today_weekday().name_cn.to_string();

    let items = match item_result.get(&weekday) {
        Some(v) if !v.is_empty() => v,
        Some(_) => {
            info!("任务结果中今日 ({weekday}) 没有可插入的数据");
            return Ok(());
        }
        None => {
            return Err(anyhow::anyhow!(
                "获取更新数据失败：结果中不含 weekday={weekday}"
            ));
        }
    };

    for item in items {
        handle_item(item, &pool).await?;
    }
    Ok(())
}

async fn handle_item(item: &TaskItem, pool: &PgPool) -> anyhow::Result<()> {
    match item {
        TaskItem::News(news) => {
            upsert_news_info(news, pool).await?;
        }
        TaskItem::Health(health) => {
            info!("健康检测结果: {} => {}", health.url, health.result);
        }
        TaskItem::ExtractNewsItem(res) => {
            info!("新闻item提取结果: {} => {}", res.url, res.result);
        }
        TaskItem::ExtractNewsNewsKeywords(res) => {
            info!("新闻keywords提取结果: {} => {}", res.url, res.result);
        }
        TaskItem::ExtractNewsEvent(res) => {
            info!("新闻event提取结果: {} => {}", res.url, res.result);
        }
        TaskItem::MergeNewsItem(res) => {
            info!("新闻event合并结果: {} => {}", res.url, res.result);
        }
    }
    Ok(())
}

fn create_empty_query() -> web::Query<QueryPage<TaskFilter>> {
    let filter = TaskFilter {
        name: None,
        arg: None,
        cmd: None,
        is_enabled: Option::from(true),
    };

    let query_page = QueryPage {
        page: None,
        filter: Some(filter),
        page_size: None,
    };

    web::Query(query_page)
}
