use crate::common::AppState;
use actix_web::{HttpResponse, post, web};
use common::api::{ApiError, ApiResponse};
use common::po::ApiResult;
use cron::Schedule;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskReq {
    pub name: String,
    pub cron: String,
    pub params: Value,
    pub retry_times: i32,
}

fn parse_retry_times(retry_times: i32) -> Result<i16, ApiError> {
    let retry_times = u8::try_from(retry_times)
        .map_err(|_| ApiError::BadRequest("retryTimes 超出范围，必须在 0-255 之间".into()))?;
    Ok(i16::from(retry_times))
}

fn validate_task_req(req: &TaskReq) -> Result<(), ApiError> {
    if req.name.trim().is_empty() {
        return Err(ApiError::BadRequest("name 不能为空".into()));
    }

    if req.cron.trim().is_empty() {
        return Err(ApiError::BadRequest("cron 不能为空".into()));
    }

    Schedule::from_str(req.cron.trim())
        .map_err(|_| ApiError::BadRequest("cron 表达式不合法".into()))?;

    Ok(())
}

#[post("/sync/task_source")]
async fn sync_task_source(req: web::Json<TaskReq>, app_state: web::Data<AppState>) -> ApiResult {
    validate_task_req(&req)?;
    let retry_times = parse_retry_times(req.retry_times)?;

    sqlx::query(
        r#"
        INSERT INTO scheduled_tasks  (name, cron, params, retry_times)
        values($1, $2, $3, $4)
        ON CONFLICT (name) DO UPDATE SET
            cron = EXCLUDED.cron,
            params = EXCLUDED.params,
            retry_times = EXCLUDED.retry_times
        "#,
    )
    .bind(req.name.clone())
    .bind(req.cron.clone())
    .bind(req.params.clone())
    .bind(retry_times)
    .execute(&app_state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("插入或更新 scheduled_tasks {:?} 失败: {}", req, e);
        ApiError::Internal("同步定时任务配置失败".into())
    })?;

    app_state.task_manager.refresh_config().await.map_err(|e| {
        tracing::error!("刷新定时任务配置失败: {e:?}");
        ApiError::Internal("定时任务配置刷新失败".into())
    })?;

    Ok(HttpResponse::Ok().json(ApiResponse::ok(serde_json::json!({
        "message": "同步成功",
    }))))
}

#[cfg(test)]
mod tests {
    use super::{TaskReq, parse_retry_times, validate_task_req};

    fn sample_req() -> TaskReq {
        TaskReq {
            name: "news_task".to_string(),
            cron: "0 */5 * * * * *".to_string(),
            params: serde_json::json!({"arg": "x"}),
            retry_times: 3,
        }
    }

    #[test]
    fn parse_retry_times_accepts_u8_range() {
        assert_eq!(parse_retry_times(0).unwrap(), 0);
        assert_eq!(parse_retry_times(255).unwrap(), 255);
    }

    #[test]
    fn parse_retry_times_rejects_out_of_range_values() {
        assert!(parse_retry_times(-1).is_err());
        assert!(parse_retry_times(256).is_err());
    }

    #[test]
    fn validate_task_req_accepts_non_empty_name_and_cron() {
        let req = sample_req();
        assert!(validate_task_req(&req).is_ok());
    }

    #[test]
    fn validate_task_req_rejects_blank_name() {
        let mut req = sample_req();
        req.name = "   ".to_string();
        assert!(validate_task_req(&req).is_err());
    }

    #[test]
    fn validate_task_req_rejects_blank_cron() {
        let mut req = sample_req();
        req.cron = "\t".to_string();
        assert!(validate_task_req(&req).is_err());
    }

    #[test]
    fn validate_task_req_rejects_invalid_cron() {
        let mut req = sample_req();
        req.cron = "invalid cron".to_string();
        assert!(validate_task_req(&req).is_err());
    }
}
