use crate::common::AppState;
use actix_web::{HttpRequest, HttpResponse, post, web};
use common::api::{ApiError, ApiResponse};
use common::dto::CreateScheduledTaskDTO;
use common::po::ApiResult;
use infra::create_scheduled_task;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateScheduledTaskReq {
    name: String,
    cron: String,
    params: serde_json::Value,
    #[serde(default = "bool::default")]
    is_enabled: bool,
    #[serde(default = "default_retry_times")]
    retry_times: i32,
}

fn default_retry_times() -> i32 {
    3
}

fn parse_retry_times(retry_times: i32) -> Result<u8, ApiError> {
    u8::try_from(retry_times)
        .map_err(|_| ApiError::BadRequest("retryTimes 超出范围，必须在 0-255 之间".into()))
}

fn into_create_dto(req: CreateScheduledTaskReq) -> Result<CreateScheduledTaskDTO, ApiError> {
    Ok(CreateScheduledTaskDTO {
        name: req.name,
        cron: req.cron,
        params: req.params,
        is_enabled: req.is_enabled,
        retry_times: parse_retry_times(req.retry_times)?,
    })
}

#[post("/scheduledTasks")]
async fn scheduled_tasks_create(
    req: HttpRequest,
    body: web::Json<CreateScheduledTaskReq>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    super::ensure_admin_access(&req, &app_state).await?;
    let dto = into_create_dto(body.into_inner())?;

    match create_scheduled_task(&dto, &app_state.db_pool).await {
        Ok(task) => {
            app_state.task_manager.refresh_config().await.map_err(|e| {
                tracing::error!("定时任务调度器刷新失败: {e:?}");
                ApiError::Internal("定时任务配置刷新失败".into())
            })?;
            Ok(HttpResponse::Created().json(ApiResponse::ok(task)))
        }
        Err(e) => {
            tracing::error!("创建定时任务失败: {e:?}");
            Err(ApiError::Internal("创建定时任务失败".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_retry_times;

    #[test]
    fn parse_retry_times_rejects_out_of_range_values() {
        assert!(parse_retry_times(-1).is_err());
        assert!(parse_retry_times(256).is_err());
    }
}
