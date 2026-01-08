use crate::common::{AppState, ExtractToken};
use actix_web::{HttpRequest, HttpResponse, post, web};
use chrono::Utc;
use common::api::{ApiError, ApiResponse};
use common::po::ApiResult;
use common::utils::verify_jwt;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, FromRow)]
struct UserSettings {
    id: i64,
    user_id: i64,
    setting_type: String,
    data: Option<serde_json::Value>,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Setting {
    setting_type: String,
    data: Option<serde_json::Value>,
}

#[post("/sync/me")]
async fn sync_me_post(
    req: HttpRequest,
    body: web::Json<Setting>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    // 获取请求体
    let body = body.into_inner();

    if let Some(token) = req.get_access_token()
        && let Ok(claims) = verify_jwt(&token)
        && !body.setting_type.is_empty()
        && body.data.is_some()
    {
        let _ = sqlx::query_as::<_, UserSettings>(
            r#"
                INSERT INTO user_setting (
                    user_id,
                    setting_type,
                    data
                ) VALUES ($1, $2, $3)
                ON CONFLICT (user_id, setting_type) DO UPDATE SET
                    data = EXCLUDED.data;
            "#,
        )
        .bind(claims.uid)
        .bind(body.setting_type)
        .bind(body.data)
        .fetch_optional(&app_state.db_pool)
        .await
        .map_err(|e| {
            tracing::error!("同步用户配置失败: {e}");
            ApiError::Internal("服务器内部错误".into())
        })?;
        return Ok(HttpResponse::Ok().json(ApiResponse::ok("数据同步成功")));
    }
    Err(ApiError::Unauthorized("请求参数不正确".into()))
}
