use crate::common::AppState;
use actix_web::{HttpRequest, HttpResponse, delete, web};
use common::api::{ApiError, ApiResponse};
use common::po::ApiResult;
use infra::delete_scheduled_task;

#[delete("/scheduledTasks/{id}")]
async fn scheduled_tasks_delete(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    super::ensure_admin_access(&req, &app_state).await?;
    let id = path.into_inner();
    match delete_scheduled_task(id, &app_state.db_pool).await {
        Ok(()) => {
            if let Err(e) = app_state.task_manager.refresh_config().await {
                tracing::warn!("定时任务调度器刷新失败: {e:?}");
            }
            Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(())))
        }
        Err(e) => {
            tracing::error!("删除定时任务 {id} 失败: {e:?}");
            Err(ApiError::NotFound(format!("任务 {id} 不存在")))
        }
    }
}
