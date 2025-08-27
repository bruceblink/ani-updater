pub mod commands;
pub mod scheduler;
pub mod task;
use crate::service::task_service::run_task_service;
use crate::timer_tasker::commands::build_cmd_map;
use crate::timer_tasker::scheduler::Scheduler;
use crate::timer_tasker::task::{TaskMeta, TaskResult, build_tasks_from_meta};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::warn;

/// 启动异步定时任务
pub async fn start_async_timer_task(task_metas: Vec<TaskMeta>, connect_pool: PgPool) {
    // 2) 构建命令表（CmdFn 映射）
    let cmd_map = build_cmd_map();
    // 3) 从 metas -> 运行时 Tasks
    let tasks = build_tasks_from_meta(&task_metas, &cmd_map);
    // 4) 创建 Scheduler（内部使用 Arc<Task> 等）
    let scheduler = Scheduler::new(tasks, None);
    let scheduler_arc = Arc::new(scheduler);

    // 6) 创建 mpsc channel 用于接收 TaskResult
    let (tx, mut rx) = mpsc::channel::<TaskResult>(128);
    let connect_pool = Arc::new(connect_pool);
    // 8) 启动结果接收器（异步）
    tokio::spawn({
        async move {
            while let Some(res) = rx.recv().await {
                if let Some(ani_item_result) = res.result {
                    let connect_pool = Arc::clone(&connect_pool);
                    tokio::spawn(async move {
                        if let Err(e) = run_task_service(ani_item_result, connect_pool).await {
                            warn!("task {:?} 保存失败", e);
                        }
                    });
                }
            }
        }
    });

    // 9) 启动调度器（异步）
    tokio::spawn({
        let scheduler_run = scheduler_arc.clone();
        async move {
            scheduler_run.run(tx).await;
        }
    });
}
