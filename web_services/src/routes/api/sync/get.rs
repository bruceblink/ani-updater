use crate::common::AppState;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, get, web};
use chrono::Utc;
use common::api::ApiError;
use common::po::ApiResult;
use common::utils::JwtClaims;
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UserSettingsDTO {
    data: Option<serde_json::Value>,
    updated_time: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Setting {
    setting_type: String,
}

#[get("/sync/me")]
async fn sync_me_get(
    req: HttpRequest,
    query: web::Query<Setting>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    let claims = req
        .extensions()
        .get::<JwtClaims>()
        .cloned()
        .ok_or_else(|| ApiError::Unauthorized("未授权".into()))?;

    let setting_type = query.setting_type.trim().to_string();
    if setting_type.is_empty() {
        return Err(ApiError::InvalidData("setting_type 不能为空".into()));
    }

    {
        let rec = sqlx::query_as::<_, UserSettings>(
            r#"
            SELECT us.id, us.user_id, us.setting_type, us.data, us.updated_at
            FROM user_setting us
            WHERE us.user_id = $1 AND us.setting_type = $2;
        "#,
        )
        .bind(claims.uid)
        .bind(&setting_type)
        .fetch_optional(&app_state.db_pool)
        .await
        .map_err(|e| {
            tracing::error!("同步用户配置失败: {e}");
            ApiError::Internal("服务器内部错误".into())
        })?;
        // 转换时间字段到上海时区
        let user_settings_dto = rec.map(|setting| UserSettingsDTO {
            data: setting.data,
            updated_time: setting.updated_at.timestamp() as u64,
        });
        return Ok(HttpResponse::Ok().json(user_settings_dto));
    }
}
