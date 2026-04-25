use crate::common::AppState;
use actix_web::{HttpResponse, put, web};
use common::api::{ApiError, ApiResponse};
use common::dto::UpdateScheduledTaskDTO;
use common::po::ApiResult;
use infra::update_scheduled_task;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateScheduledTaskReq {
    name: Option<String>,
    cron: Option<String>,
    params: Option<serde_json::Value>,
    retry_times: Option<i32>,
}

fn parse_retry_times(retry_times: i32) -> Result<u8, ApiError> {
    u8::try_from(retry_times)
        .map_err(|_| ApiError::BadRequest("retryTimes 超出范围，必须在 0-255 之间".into()))
}

fn into_update_dto(req: UpdateScheduledTaskReq) -> Result<UpdateScheduledTaskDTO, ApiError> {
    Ok(UpdateScheduledTaskDTO {
        name: req.name,
        cron: req.cron,
        params: req.params,
        retry_times: req.retry_times.map(parse_retry_times).transpose()?,
    })
}

#[put("/scheduledTasks/{id}")]
async fn scheduled_tasks_update(
    path: web::Path<i64>,
    body: web::Json<UpdateScheduledTaskReq>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    let id = path.into_inner();
    let dto = into_update_dto(body.into_inner())?;

    match update_scheduled_task(id, &dto, &app_state.db_pool).await {
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

#[cfg(test)]
mod tests {
    use super::{UpdateScheduledTaskReq, into_update_dto};

    #[test]
    fn into_update_dto_rejects_out_of_range_retry_times() {
        let req = UpdateScheduledTaskReq {
            name: None,
            cron: None,
            params: None,
            retry_times: Some(256),
        };

        assert!(into_update_dto(req).is_err());
    }
}
