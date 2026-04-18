use crate::common::AppState;
use actix_web::{HttpResponse, post, web};
use common::api::{ApiError, ApiResponse};
use common::dto::CreateScheduledTaskDTO;
use common::po::ApiResult;
use infra::create_scheduled_task;

#[post("/scheduledTasks")]
async fn scheduled_tasks_create(
    body: web::Json<CreateScheduledTaskDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    match create_scheduled_task(&body, &app_state.db_pool).await {
        Ok(task) => {
            // 重新加载定时任务调度器
            if let Err(e) = app_state.task_manager.refresh_config().await {
                tracing::warn!("定时任务调度器刷新失败: {e:?}");
            }
            Ok(HttpResponse::Created().json(ApiResponse::ok(task)))
        }
        Err(e) => {
            tracing::error!("创建定时任务失败: {e:?}");
            Err(ApiError::Internal("创建定时任务失败".into()))
        }
    }
}
