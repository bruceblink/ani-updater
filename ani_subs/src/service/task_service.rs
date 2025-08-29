use crate::dao::upsert_ani_info;
use ani_spiders::{AniItemResult, ApiResponse};
use common::utils::date_utils::get_today_weekday;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use timer_tasker::commands::build_cmd_map;
use timer_tasker::scheduler::Scheduler;
use timer_tasker::task::TaskMeta;
use timer_tasker::task::TaskResult;
use timer_tasker::task::build_tasks_from_meta;
use tokio::sync::mpsc;
use tracing::warn;

pub async fn run_task_service(
    ani_item_result: AniItemResult,
    pool: Arc<PgPool>,
) -> anyhow::Result<ApiResponse, String> {
    // 启动定时任务服务
    let weekday = get_today_weekday().name_cn.to_string();

    let items = match ani_item_result.get(&weekday) {
        Some(v) if !v.is_empty() => v,
        Some(_) => return Ok(ApiResponse::ok(json!({ "message": "没有可插入的数据" }))),
        None => return Ok(ApiResponse::err("获取今日动漫数据失败")),
    };

    for item in items {
        if let Err(e) = upsert_ani_info(item, &pool).await {
            return Ok(ApiResponse::err(format!("插入失败：{}", e)));
        }
    }
    Ok(ApiResponse::ok(json!({ "message": "save success" })))
}

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
