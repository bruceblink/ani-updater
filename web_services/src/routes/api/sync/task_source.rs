use crate::common::AppState;
use actix_web::{HttpResponse, post, web};
use common::api::{ApiError, ApiResponse};
use common::po::ApiResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskReq {
    pub name: String,
    pub cron: String,
    pub params: Value,
    pub retry_times: i32,
}

#[post("/sync/task_source")]
async fn sync_task_source(req: web::Json<TaskReq>, app_state: web::Data<AppState>) -> ApiResult {
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
    .bind(req.retry_times)
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
