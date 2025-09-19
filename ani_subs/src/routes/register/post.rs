use actix_web::{HttpResponse, Responder, web};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RegisterRequest {
    #[allow(dead_code)]
    username: String, // 用户名
    #[allow(dead_code)]
    password1: String, // 密码
    #[allow(dead_code)]
    password2: String, // 确认密码
    #[allow(dead_code)]
    email: String, // 邮箱
}

#[allow(dead_code)]
pub async fn register(
    _db_pool: web::Data<sqlx::PgPool>,
    _credentials: web::Json<RegisterRequest>,
) -> impl Responder {
    // 查询用户
    HttpResponse::Ok().json(serde_json::json!({ "res": "" }))
}
