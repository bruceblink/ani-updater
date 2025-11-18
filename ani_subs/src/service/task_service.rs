use crate::dao::{list_all_scheduled_tasks, upsert_ani_info, upsert_news_info, upsert_video_info};
use common::api::ApiResponse;
use common::api::{ItemResult, TaskItem};
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

/// 启动异步定时任务
pub async fn start_async_timer_task(connect_pool: PgPool) {
    let connect_pool = Arc::new(connect_pool);
    // 1) 初始化 定时任务配置
    let task_conf = init_scheduled_tasks_config(connect_pool.as_ref()).await;
    // 2) 构建命令表（CmdFn 映射）
    let cmd_map = build_cmd_map();
    // 3) 从 metas -> 运行时 Tasks
    let tasks = build_tasks_from_meta(&task_conf, &cmd_map);
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

/// 初始化定时任务配置
pub async fn init_scheduled_tasks_config(db_pool: &PgPool) -> Vec<TaskMeta> {
    match list_all_scheduled_tasks(db_pool).await {
        Ok(timer_tasker) => {
            timer_tasker
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
            tracing::error!("定时任务初始化配置错误: {e:?}");
            Vec::new()
        }
    }
}
