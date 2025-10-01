use crate::common::ExtractToken;
use actix_web::{HttpRequest, HttpResponse, get, web};
use chrono::Utc;
use common::api::{ApiError, ApiResponse, ApiResult};
use common::utils::verify_jwt;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Serialize, FromRow)]
struct UserSettings {
    id: i64,
    user_id: i64,
    setting_type: String,
    data: Option<serde_json::Value>,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Deserialize)]
struct Setting {
    setting_type: String,
}

#[get("/sync/me")]
async fn sync_me_get(
    req: HttpRequest,
    query: web::Query<Setting>,
    db: web::Data<PgPool>,
) -> ApiResult {
    // 获取请求参数
    let setting_type = &query.setting_type.trim();

    if let Some(token) = req.get_access_token()
        && let Ok(claims) = verify_jwt(&token)
        && !setting_type.is_empty()
    {
        let rec = sqlx::query_as::<_, UserSettings>(
            r#"
            SELECT us.id, us.user_id, us.setting_type, us.data, us.updated_at
            FROM user_setting us
            WHERE us.id = $1 AND us.setting_type = $2;
        "#,
        )
        .bind(claims.id)
        .bind(setting_type)
        .fetch_optional(db.get_ref())
        .await
        .map_err(|e| {
            tracing::error!("同步用户配置失败: {e}");
            ApiError::Internal("服务器内部错误".into())
        })?;
        return Ok(HttpResponse::Ok().json(ApiResponse::ok(rec)));
    }
    Err(ApiError::Unauthorized("请求参数不正确".into()))
}
