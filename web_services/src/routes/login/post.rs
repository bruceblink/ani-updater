use crate::common::{AppState, ExtractToken};
use actix_web::cookie::{Cookie, SameSite};
use actix_web::{HttpRequest, HttpResponse, Responder, post, web};
use bcrypt::verify;
use chrono::Utc;
use infra::get_user_by_email;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    sub: i64,   // 用户 ID
    exp: usize, // 过期时间
}

pub async fn login(
    app_state: web::Data<AppState>,
    credentials: web::Json<LoginRequest>,
) -> impl Responder {
    // 查询用户
    let user = match get_user_by_email(credentials.email.clone(), &app_state.db_pool).await {
        Ok(Some(u)) => u,
        _ => return HttpResponse::Unauthorized().finish(),
    };

    // 验证密码
    if !verify(&credentials.password, &user.password).unwrap_or(false) {
        return HttpResponse::Unauthorized().finish();
    }

    // 生成 JWT
    let claims = Claims {
        sub: user.id,
        exp: (Utc::now().timestamp() + 3600) as usize, // 1小时过期
    };

    let secret_str = std::env::var("JWT_SECRET").unwrap_or_else(|_| "mysecret".to_string());
    let secret = EncodingKey::from_secret(secret_str.as_bytes());

    let token = match encode(&Header::default(), &claims, &secret) {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    HttpResponse::Ok().json(serde_json::json!({ "token": token }))
}

#[post("/logout")]
async fn logout(app_state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    // 1️⃣ 注销 refresh_token（数据库标记为 revoked）
    if let Some(refresh_token) = req.get_refresh_token() {
        if let Err(e) = sqlx::query(
            r#"
                UPDATE refresh_tokens SET revoked = true WHERE token = $1;
            "#,
        )
        .bind(refresh_token.clone())
        .execute(&app_state.db_pool)
        .await
        {
            tracing::error!("注销 refresh_token {refresh_token:?} 失败: {e}");
        }
    } else {
        tracing::warn!("用户登出时未携带 refresh_token");
    }

    // 2️⃣ 清空 access_token 和 refresh_token cookie
    let expired_cookie = |name: String| {
        Cookie::build(name, "")
            .path("/")
            .http_only(true)
            .secure(true)
            .same_site(SameSite::None)
            .expires(time::OffsetDateTime::now_utc() - time::Duration::seconds(1))
            .finish()
    };

    let access_cookie = expired_cookie("access_token".to_string());
    let refresh_cookie = expired_cookie("refresh_token".to_string());

    // 3️⃣ 返回 204 No Content 更语义化
    HttpResponse::NoContent()
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .finish()
}
