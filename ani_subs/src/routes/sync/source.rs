use actix_web::{HttpResponse, post, web};
use common::api::{ApiResponse, ApiResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskReq {
    pub name: String,
    pub cron: String,
    pub params: Value,    // 任意 JSON
    pub retry_times: i32, // 如果次数不大也可以用 i16
}

// 同步news数据源的 API
#[post("/sync/source")]
async fn sync_task_source(
    db_pool: web::Data<PgPool>,
    req: web::Json<TaskReq>, // 接收任意 JSON
) -> ApiResult {
    let _ = sqlx::query(
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
    .execute(db_pool.get_ref())
    .await
    .map_err(|e| {
        tracing::error!("插入或更新 scheduled_tasks {:?} 失败: {}", req, e);
        anyhow::anyhow!(e)
    })?;

    Ok(HttpResponse::Ok().json(ApiResponse::ok(serde_json::json!({
        "message": "同步成功",
    }))))
}
