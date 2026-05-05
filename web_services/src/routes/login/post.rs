use crate::common::{AppState, ExtractToken};
use actix_web::cookie::{Cookie, SameSite};
use actix_web::{HttpRequest, HttpResponse, Responder, post, web};
use chrono::Utc;
use common::api::{ApiError, ApiResponse};
use common::utils::{CommonUser, generate_jwt, generate_refresh_token};
use common::{ACCESS_TOKEN, REFRESH_TOKEN};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

fn token_window_days(
    app_state: &web::Data<AppState>,
    token_key: &'static str,
) -> Result<i64, ApiError> {
    app_state
        .configuration
        .token
        .get(token_key)
        .copied()
        .ok_or_else(|| {
            tracing::error!("token 配置缺失: {token_key}");
            ApiError::Internal("token 配置缺失".into())
        })
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: Option<String>,
    pub email: Option<String>,
    pub password: String,
}

#[derive(Debug, Serialize, FromRow)]
struct LoginUser {
    id: i64,
    email: String,
    username: String,
    password: String,
    avatar_url: Option<String>,
    token_version: i64,
    status: String,
}

#[post("/login")]
async fn login(
    app_state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, ApiError> {
    let req = body.into_inner();
    if req.password.len() < 8 {
        return Err(ApiError::InvalidData("用户名/邮箱或密码错误".into()));
    }

    let username = req
        .username
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let email = req
        .email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    if username.is_none() && email.is_none() {
        return Err(ApiError::InvalidData("请提供 username 或 email".into()));
    }

    let user = sqlx::query_as::<_, LoginUser>(
        r#"
            SELECT id, email, username, password, avatar_url, token_version, status
            FROM user_info
            WHERE ($1::text IS NULL OR username = $1)
              AND ($2::text IS NULL OR email = $2)
            ORDER BY id DESC
            LIMIT 1
        "#,
    )
    .bind(username)
    .bind(email)
    .fetch_optional(&app_state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("查询登录用户失败: {e}");
        ApiError::Internal("服务器内部错误".into())
    })?
    .ok_or_else(|| ApiError::Unauthorized("用户名/邮箱或密码错误".into()))?;

    if user.status != "active" {
        return Err(ApiError::Forbidden("账号不可用".into()));
    }

    let verify_ok = bcrypt::verify(&req.password, &user.password).map_err(|e| {
        tracing::error!("校验密码失败: {e}");
        ApiError::Internal("服务器内部错误".into())
    })?;

    if !verify_ok {
        return Err(ApiError::Unauthorized("用户名/邮箱或密码错误".into()));
    }

    let roles: Vec<String> = sqlx::query_scalar(
        r#"
            SELECT r.name
            FROM roles r
            JOIN user_roles ur ON ur.role_id = r.id
            WHERE ur.user_id = $1
        "#,
    )
    .bind(user.id)
    .fetch_all(&app_state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("查询用户角色失败: {e}");
        ApiError::Internal("服务器内部错误".into())
    })?;

    let access_token_mins = token_window_days(&app_state, ACCESS_TOKEN)?;
    let refresh_token_days = token_window_days(&app_state, REFRESH_TOKEN)?;

    let common_user = CommonUser {
        id: user.id,
        sub: user.username.clone(),
        uid: user.id,
        email: Some(user.email.clone()),
        avatar_url: user.avatar_url,
        r#type: "local".to_string(),
        roles,
        ver: user.token_version,
    };

    let access_token = generate_jwt(&common_user, access_token_mins).map_err(|e| {
        tracing::error!("access token 生成失败: {e}");
        ApiError::Internal("access token 生成失败".into())
    })?;

    let refresh_token = generate_refresh_token(refresh_token_days).map_err(|e| {
        tracing::error!("refresh token 生成失败: {e}");
        ApiError::Internal("refresh token 生成失败".into())
    })?;

    let session_expires_at = Utc::now() + chrono::Duration::days(30);

    sqlx::query(
        r#"
            INSERT INTO refresh_tokens (user_id, token, expires_at, session_expires_at)
            VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(user.id)
    .bind(&refresh_token.token)
    .bind(refresh_token.expires_at)
    .bind(session_expires_at)
    .execute(&app_state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("持久化 refresh_token 失败: {e}");
        ApiError::Internal("服务器内部错误".into())
    })?;

    let is_prod = app_state.configuration.is_production;
    let access_cookie = Cookie::build(ACCESS_TOKEN, access_token.token.clone())
        .http_only(true)
        .secure(is_prod)
        .same_site(SameSite::None)
        .path("/")
        .finish();

    let refresh_cookie = Cookie::build(REFRESH_TOKEN, refresh_token.token)
        .http_only(true)
        .secure(is_prod)
        .same_site(SameSite::None)
        .path("/")
        .finish();

    Ok(HttpResponse::Ok()
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .json(ApiResponse::ok(serde_json::json!({
            "access_token_exp": access_token.expires_at.timestamp(),
            "user": common_user
        }))))
}

#[post("/logout")]
async fn logout(app_state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    if let Some(refresh_token) = req.get_refresh_token() {
        if let Err(e) = sqlx::query(
            r#"
                UPDATE refresh_tokens SET revoked = true WHERE token = $1;
            "#,
        )
        .bind(refresh_token)
        .execute(&app_state.db_pool)
        .await
        {
            tracing::error!("注销 refresh_token 失败: {e}");
        }
    } else {
        tracing::warn!("用户登出时未携带 refresh_token");
    }

    let is_prod = app_state.configuration.is_production;
    let expired_cookie = |name: String| {
        Cookie::build(name, "")
            .path("/")
            .http_only(true)
            .secure(is_prod)
            .same_site(SameSite::None)
            .expires(time::OffsetDateTime::now_utc() - time::Duration::seconds(1))
            .finish()
    };

    let access_cookie = expired_cookie("access_token".to_string());
    let refresh_cookie = expired_cookie("refresh_token".to_string());

    HttpResponse::NoContent()
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .finish()
}
