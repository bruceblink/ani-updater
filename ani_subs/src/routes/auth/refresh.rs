use crate::common::{ACCESS_TOKEN, REFRESH_TOKEN};
use crate::configuration::Setting;
use actix_web::cookie::Cookie;
use actix_web::{HttpRequest, HttpResponse, post, web};
use common::api::{ApiError, ApiResponse, ApiResult};
use common::utils::{GithubUser, generate_jwt, generate_refresh_token};
use serde::Serialize;
use sqlx::{FromRow, PgPool};

#[derive(Debug, Serialize, FromRow)]
struct UserWithIdentity {
    id: i64,
    email: Option<String>,
    username: Option<String>,
    display_name: Option<String>,
    avatar_url: Option<String>,
    provider: Option<String>,
    provider_uid: Option<String>,
}

/**
    access_token刷新的API，cookie中携带refresh token获取access_token <br>
    /auth/refresh  POST请求
*/
#[post("/auth/refresh")]
async fn auth_refresh(
    req: HttpRequest,
    db: web::Data<PgPool>,
    config: web::Data<Setting>,
) -> ApiResult {
    let old_refresh_cookie = req
        .cookie(REFRESH_TOKEN)
        .ok_or_else(|| ApiError::Unauthorized("缺少 refresh token".into()))?;
    let old_refresh_token = old_refresh_cookie.value();

    let new_refresh_token = generate_refresh_token(config.token[REFRESH_TOKEN] as i64)
        .map_err(|_| ApiError::Internal("refresh_token 生成失败".into()))?;

    let rec = sqlx::query_as::<_, UserWithIdentity>(
        r#"
            WITH valid_token AS (
                SELECT user_id
                FROM refresh_tokens
                WHERE token = $1 AND expires_at > now() AND revoked = false
            ),
            deleted AS (
                DELETE FROM refresh_tokens
                WHERE token = $1
                RETURNING user_id
            ),
            inserted AS (
                INSERT INTO refresh_tokens (user_id, token, expires_at)
                SELECT user_id, $2, now() + interval '30 days'
                FROM valid_token
                RETURNING user_id
            )
            SELECT ui.id, ui.email, ui.username, ui.display_name, ui.avatar_url,
                   uident.provider, uident.provider_uid
            FROM user_info ui
            JOIN valid_token vt ON ui.id = vt.user_id
            LEFT JOIN user_identities uident ON ui.id = uident.user_id
        "#,
    )
    .bind(old_refresh_token)
    .bind(&new_refresh_token.token)
    .fetch_optional(db.get_ref())
    .await
    .map_err(|e| {
        tracing::error!("刷新 token 查询失败: {e}");
        ApiError::Internal("服务器错误".into())
    })?;

    let github_user = match rec {
        Some(u) => GithubUser {
            login: u.username.unwrap_or_default(),
            id: u.provider_uid.unwrap_or_default().parse().unwrap_or(0),
            avatar_url: u.avatar_url,
            name: u.display_name,
            email: u.email,
        },
        None => return Err(ApiError::Unauthorized("refresh token 无效或已过期".into())),
    };

    let new_access_token = generate_jwt(&github_user, config.token[ACCESS_TOKEN] as i64)
        .map_err(|_| ApiError::Unauthorized("refresh token 无效或已过期".into()))?;

    let access_cookie = Cookie::build(ACCESS_TOKEN, new_access_token.token.clone())
        .http_only(true)
        .secure(true)
        .path("/")
        .same_site(actix_web::cookie::SameSite::None)
        .finish();

    let refresh_cookie = Cookie::build(REFRESH_TOKEN, new_refresh_token.token.clone())
        .http_only(true)
        .secure(true)
        .path("/")
        .same_site(actix_web::cookie::SameSite::None)
        .finish();

    Ok(HttpResponse::Ok()
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .json(ApiResponse::ok(serde_json::json!({
            "message": "刷新成功",
            "access_token": new_access_token.token,
            "access_token_exp": new_access_token.expires_at.timestamp() as usize,
            "user": github_user
        }))))
}
