use crate::common::AppState;
use actix_web::cookie::Cookie;
use actix_web::{HttpRequest, HttpResponse, post, web};
use chrono::Utc;
use common::api::{ApiError, ApiResponse};
use common::po::ApiResult;
use common::utils::{CommonUser, generate_jwt, generate_refresh_token};
use common::{ACCESS_TOKEN, REFRESH_TOKEN};
use serde::Serialize;
use sqlx::FromRow;

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

#[derive(Debug, Serialize, FromRow)]
struct RefreshTokens {
    user_id: i64,
    session_expires_at: chrono::DateTime<Utc>,
}

///
/// 刷新access_token的API，cookie中携带refresh token获取access_token <br>
/// /auth/token/refresh  POST请求
///
#[post("/auth/token/refresh")]
async fn auth_token_refresh(req: HttpRequest, app_state: web::Data<AppState>) -> ApiResult {
    let refresh_cookie = req
        .cookie(REFRESH_TOKEN)
        .ok_or_else(|| ApiError::Unauthorized("缺少 refresh token".into()))?;

    let old_refresh_token = refresh_cookie.value();

    let mut tx = app_state.db_pool.begin().await.map_err(|e| {
        tracing::error!("begin tx failed: {e}");
        ApiError::Internal("服务器错误".into())
    })?;

    // 1️⃣ 校验并消费旧 refresh_token
    let rec = sqlx::query_as::<_, RefreshTokens>(
        r#"
            SELECT user_id, session_expires_at
            FROM refresh_tokens
            WHERE token = $1
              AND revoked = false
              AND expires_at > now()
              AND session_expires_at > now()
            FOR UPDATE
        "#,
    )
    .bind(old_refresh_token)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("query refresh_token failed: {e}");
        ApiError::Internal("服务器错误".into())
    })?;

    let rec = match rec {
        Some(r) => r,
        None => {
            tx.rollback().await.ok();
            return Err(ApiError::Unauthorized("refresh token 无效或已过期".into()));
        }
    };

    let user_id = rec.user_id;
    let session_expires_at = rec.session_expires_at;

    // 2️⃣ 立即吊销旧 token（防并发重放）
    sqlx::query(
        r#"
            UPDATE refresh_tokens
            SET revoked = true
            WHERE token = $1
        "#,
    )
    .bind(old_refresh_token)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("revoke refresh_token failed: {e}");
        ApiError::Internal("服务器错误".into())
    })?;

    // 3️⃣ 生成新 refresh_token（滑动窗口 + 终点）
    let refresh_window_days = app_state.configuration.token[REFRESH_TOKEN];
    let now = Utc::now();

    let mut new_expires_at = now + chrono::Duration::days(refresh_window_days);
    if new_expires_at > session_expires_at {
        new_expires_at = session_expires_at;
    }

    let new_refresh_token = generate_refresh_token(refresh_window_days)
        .map_err(|_| ApiError::Internal("refresh_token 生成失败".into()))?;

    sqlx::query(
        r#"
            INSERT INTO refresh_tokens
                (user_id, token, expires_at, session_expires_at)
            VALUES
                ($1, $2, $3, $4)
        "#,
    )
    .bind(user_id)
    .bind(new_refresh_token.clone().token)
    .bind(new_expires_at)
    .bind(session_expires_at)
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        tracing::error!("insert refresh_token failed: {e}");
        ApiError::Internal("服务器错误".into())
    })?;

    // 4️⃣ 查询用户 + 角色
    let user = sqlx::query_as::<_, UserWithIdentity>(
        r#"
                SELECT
                    ui.id,
                    ui.email,
                    ui.username,
                    ui.display_name,
                    ui.avatar_url,
                    uident.provider,
                    uident.provider_uid
                FROM user_info ui
                LEFT JOIN user_identities uident ON uident.user_id = ui.id
                WHERE ui.id = $1
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| ApiError::Internal("用户不存在".into()))?;

    let roles: Vec<String> = sqlx::query_scalar(
        r#"
            SELECT r.code
            FROM roles r
            JOIN user_roles ur ON ur.role_id = r.id
            WHERE ur.user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_all(&mut *tx)
    .await
    .unwrap_or_default();

    tx.commit().await.ok();

    // 5️⃣ 生成 access_token
    let uid = user
        .provider_uid
        .as_deref()
        .unwrap_or("0")
        .parse()
        .map_err(|_| ApiError::InvalidData("provider_uid invalid".into()))?;

    let common_user = CommonUser {
        id: user.id,
        sub: user.username.unwrap_or_default(),
        uid,
        roles,
        r#type: user.provider.unwrap_or_default(),
        ver: 0,
    };

    let access_token = generate_jwt(&common_user, app_state.configuration.token[ACCESS_TOKEN])
        .map_err(|_| ApiError::Internal("access token 生成失败".into()))?;

    // 6️⃣ 写入 Cookie
    let access_cookie = Cookie::build(ACCESS_TOKEN, access_token.token.clone())
        .http_only(true)
        .secure(true)
        .same_site(actix_web::cookie::SameSite::None)
        .path("/")
        .finish();

    let refresh_cookie = Cookie::build(REFRESH_TOKEN, new_refresh_token.token)
        .http_only(true)
        .secure(true)
        .same_site(actix_web::cookie::SameSite::None)
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

/// 吊销access_token的API <br>
/// /auth/token/revoke  POST请求
#[post("/auth/token/revoke")]
async fn auth_token_revoke(_req: HttpRequest, _app_state: web::Data<AppState>) -> ApiResult {
    todo!()
}
