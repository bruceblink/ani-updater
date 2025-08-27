pub mod commands;
pub mod scheduler;
pub mod task;
use crate::configuration::load_timer_task_conf;
use crate::timer_tasker::commands::build_cmd_map;
use crate::timer_tasker::scheduler::Scheduler;
use crate::timer_tasker::task::{TaskMeta, TaskResult, build_tasks_from_meta};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::warn;

///从配置文件加载定时作业的配置数据
pub fn load_timer_tasks_config(config_path: PathBuf) -> Vec<TaskMeta> {
    let configuration = load_timer_task_conf(config_path).expect("Failed to read configuration.");
    let anime_sources = configuration
        .datasource
        .get("anime")
        .expect("Missing anime category");

    let mut tasks: Vec<TaskMeta> = Vec::new();
    for datasource in anime_sources {
        tasks.push(TaskMeta {
            name: datasource.name.clone(),
            cron_expr: datasource.cron_expr.clone(),
            cmd: datasource.cmd.clone(),
            arg: datasource.url.clone(),
            retry_times: datasource.retry_times,
        });
    }
    tasks
}

/// 启动异步定时任务
pub async fn start_async_timer_task(config_path: PathBuf) {
    // 1) 构造/加载配置
    let task_metas = load_timer_tasks_config(config_path);
    // 2) 构建命令表（CmdFn 映射）
    let cmd_map = build_cmd_map();
    // 3) 从 metas -> 运行时 Tasks
    let tasks = build_tasks_from_meta(&task_metas, &cmd_map);
    // 4) 创建 Scheduler（内部使用 Arc<Task> 等）
    let scheduler = Scheduler::new(tasks, None);
    let scheduler_arc = Arc::new(scheduler);

    // 6) 创建 mpsc channel 用于接收 TaskResult
    let (tx, mut rx) = mpsc::channel::<TaskResult>(128);

    // 8) 启动结果接收器（异步）
    tokio::spawn({
        async move {
            while let Some(res) = rx.recv().await {
                if let Some(ani_item_result) = res.result {
                    //let db = state_for_loop.db.clone(); // Arc<SqlitePool>
                    tokio::spawn(async move {
                        //if let Err(e) = save_ani_item_data_db(db, ani_item_result).await {
                        warn!("task {:?} 保存失败", ani_item_result);
                        //}
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
