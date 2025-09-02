use crate::dao::get_user_by_username;
use actix_web::cookie::{Cookie, SameSite};
use actix_web::{HttpResponse, Responder, post, web};
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
    db_pool: web::Data<sqlx::PgPool>,
    credentials: web::Json<LoginRequest>,
) -> impl Responder {
    // 查询用户
    let user = match get_user_by_username(credentials.username.clone(), &db_pool).await {
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
async fn logout() -> impl Responder {
    // 设置一个同名 cookie，立即过期
    let cookie = Cookie::build("access_token", "")
        .path("/")
        .http_only(true)
        .secure(true) // 之前 cookie 是 Secure
        .same_site(SameSite::None)
        .max_age(time::Duration::seconds(0)) // 设置立即过期
        .finish();

    HttpResponse::Ok().cookie(cookie).body("logged out")
}
