use crate::common::AppState;
use actix_web::{HttpResponse, Responder, post, web};
use serde::Deserialize;

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct RegisterRequest {
    username: String,  // 用户名
    password1: String, // 密码
    password2: String, // 确认密码
    email: String,     // 邮箱
}

#[post("/register")]
pub async fn register(
    _app_state: web::Data<AppState>,
    _credentials: web::Json<RegisterRequest>,
) -> impl Responder {
    // 查询用户
    HttpResponse::Ok().json(serde_json::json!({"user_id": 1001}))
}
