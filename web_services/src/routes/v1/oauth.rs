use crate::common::AppState;
use actix_web::{HttpRequest, get, post, web};
use common::po::ApiResult;

/// POST /oauth/github/authorize <br>
/// 用途 获取 GitHub 授权 URL
///
#[post("/oauth/github/authorize")]
async fn github_authorize(_req: HttpRequest, _app_state: web::Data<AppState>) -> ApiResult {
    todo!()
}

/// GET /v1/oauth/github/callback
/// 浏览器 only
#[get("/oauth/github/callback")]
async fn github_callback(_req: HttpRequest, _app_state: web::Data<AppState>) -> ApiResult {
    todo!()
}
