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
            app_state.task_manager.refresh_config().await.map_err(|e| {
                tracing::error!("定时任务调度器刷新失败: {e:?}");
                ApiError::Internal("定时任务配置刷新失败".into())
            })?;
            Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(())))
        }
        Err(e) => {
            tracing::error!("删除定时任务 {id} 失败: {e:?}");
            Err(ApiError::NotFound(format!("任务 {id} 不存在")))
        }
    }
}
