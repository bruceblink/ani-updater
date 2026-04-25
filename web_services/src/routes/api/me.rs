use crate::common::AppState;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, get, web};
use common::api::{ApiError, ApiResponse};
use common::po::ApiResult;
use common::utils::JwtClaims;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct UserView {
    id: i64,
    username: String,
    email: Option<String>,
    avatar: Option<String>,
    roles: Vec<String>,
    permissions: Vec<String>,
}

#[get("/me")]
async fn me(req: HttpRequest, app_state: web::Data<AppState>) -> ApiResult {
    let claims = req
        .extensions()
        .get::<JwtClaims>()
        .cloned()
        .ok_or_else(|| ApiError::Unauthorized("未携带或非法的 JWT".into()))?;

    let permissions: Vec<String> = sqlx::query_scalar(
        r#"
            SELECT DISTINCT p.name
            FROM permissions p
            LEFT JOIN role_permissions rp ON rp.permission_id = p.id
            LEFT JOIN user_roles ur ON ur.role_id = rp.role_id AND ur.user_id = $1
            LEFT JOIN plan_permissions pp ON pp.permission_id = p.id
            LEFT JOIN user_info ui ON ui.id = $1 AND ui.plan = pp.plan
            WHERE ur.user_id IS NOT NULL OR ui.id IS NOT NULL
            ORDER BY p.name
        "#,
    )
    .bind(claims.uid)
    .fetch_all(&app_state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("查询用户权限失败: {e}");
        ApiError::Internal("服务器内部错误".into())
    })?;

    let user = UserView {
        id: claims.uid,
        username: claims.sub,
        email: claims.email,
        avatar: claims.avatar,
        roles: claims.roles,
        permissions,
    };
    Ok(HttpResponse::Ok().json(ApiResponse::ok(user)))
}
