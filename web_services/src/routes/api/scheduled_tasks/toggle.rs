use crate::common::AppState;
use actix_web::{HttpRequest, HttpResponse, patch, web};
use common::api::{ApiError, ApiResponse};
use common::dto::ToggleScheduledTaskDTO;
use common::po::ApiResult;
use infra::toggle_scheduled_task;

#[patch("/scheduledTasks/{id}/status")]
async fn scheduled_tasks_toggle(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<ToggleScheduledTaskDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    super::ensure_admin_access(&req, &app_state).await?;
    let id = path.into_inner();
    match toggle_scheduled_task(id, &body, &app_state.db_pool).await {
        Ok(()) => {
            app_state.task_manager.refresh_config().await.map_err(|e| {
                tracing::error!("定时任务调度器刷新失败: {e:?}");
                ApiError::Internal("定时任务配置刷新失败".into())
            })?;
            Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(())))
        }
        Err(e) => {
            tracing::error!("切换定时任务 {id} 状态失败: {e:?}");
            Err(ApiError::NotFound(format!("任务 {id} 不存在")))
        }
    }
}
