use actix_web::{HttpResponse, get};
use common::api::{ApiResponse, ApiResult};

#[get("/")]
async fn index() -> ApiResult {
    Ok(HttpResponse::Ok().json(ApiResponse::ok("使用 GitHub 进行第三方登录")))
}
