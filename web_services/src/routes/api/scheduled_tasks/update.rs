use crate::common::AppState;
use actix_web::{HttpResponse, put, web};
use common::api::{ApiError, ApiResponse};
use common::dto::UpdateScheduledTaskDTO;
use common::po::ApiResult;
use infra::update_scheduled_task;

#[put("/scheduledTasks/{id}")]
async fn scheduled_tasks_update(
    path: web::Path<i64>,
    body: web::Json<UpdateScheduledTaskDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    let id = path.into_inner();
    match update_scheduled_task(id, &body, &app_state.db_pool).await {
        Ok(task) => {
            if let Err(e) = app_state.task_manager.refresh_config().await {
                tracing::warn!("定时任务调度器刷新失败: {e:?}");
            }
            Ok(HttpResponse::Ok().json(ApiResponse::ok(task)))
        }
        Err(e) => {
            tracing::error!("更新定时任务 {id} 失败: {e:?}");
            Err(ApiError::NotFound(format!("任务 {id} 不存在或更新失败")))
        }
    }
}
