use crate::common::{AppState, ExtractToken};
use crate::dao::get_user_by_username;
use actix_web::cookie::{Cookie, SameSite};
use actix_web::{HttpRequest, HttpResponse, Responder, post, web};
use bcrypt::verify;
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
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
    let user = match get_user_by_username(credentials.username.clone(), &app_state.db_pool).await {
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
    if let Some(token) = req.get_refresh_token() {
        let _ = sqlx::query(
            r#"
            UPDATE refresh_tokens SET revoked = true WHERE token = $1;
            "#,
        )
        .bind(token.clone())
        .execute(&app_state.db_pool)
        .await
        .map_err(|e| tracing::error!("token {token:?} 注销失败: {e}"));
    } else {
        tracing::warn!("用户登出时没有携带 access_token");
    }

    // 设置一个同名 cookie，立即过期， 浏览器会自动删除该cookie
    let access_cookie = Cookie::build("access_token", "")
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::None)
        .max_age(time::Duration::seconds(0))
        .finish();

    let refresh_cookie = Cookie::build("refresh_token", "")
        .path("/")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::None)
        .max_age(time::Duration::seconds(0))
        .finish();

    HttpResponse::Ok()
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .body("logged out")
}
