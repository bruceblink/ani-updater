use crate::dao::get_user_by_username;
use actix_web::{HttpResponse, Responder, web};
use bcrypt::verify;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct RegisterRequest {
    username: String,  // 用户名
    password1: String, // 密码
    password2: String, // 确认密码
    email: String,     // 邮箱
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    sub: i64,   // 用户 ID
    exp: usize, // 过期时间
}

pub async fn register(
    db_pool: web::Data<sqlx::PgPool>,
    credentials: web::Json<RegisterRequest>,
) -> impl Responder {
    // 查询用户
    let user = match get_user_by_username(credentials.username.clone(), &db_pool).await {
        Ok(Some(u)) => u,
        _ => return HttpResponse::Unauthorized().finish(),
    };

    // 验证密码
    if !verify(&credentials.password1, &user.password).unwrap_or(false) {
        return HttpResponse::Unauthorized().finish();
    }

    HttpResponse::Ok().json(serde_json::json!({ "res": "" }))
}
