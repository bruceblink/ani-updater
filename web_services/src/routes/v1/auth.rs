use crate::common::AppState;
use actix_web::post;
use actix_web::{HttpRequest, get, web};
use common::po::ApiResult;

/// GET /v1/auth/me <br>
/// 用途: 判断「当前是否已登录」,前端判断登录态<br>
#[get("/v1/auth/session")]
async fn auth_session(_req: HttpRequest, _app_state: web::Data<AppState>) -> ApiResult {
    todo!()
}

/// POST /v1/auth/logout <br>
/// 用途: 撤销当前 session <br>
/// 行为: 吊销 refresh_token, 清空cookie
#[post("/v1/auth/logout")]
async fn auth_logout(_req: HttpRequest, _app_state: web::Data<AppState>) -> ApiResult {
    todo!()
}

/// POST /v1/auth/token
/// 用途: token签发的统一入口
#[post("/v1/auth/token")]
async fn auth_token(_req: HttpRequest, _app_state: web::Data<AppState>) -> ApiResult {
    todo!()
}

/// GET /v1/auth/me
/// 用途: 用于返回用户信息
#[get("/v1/auth/me")]
async fn auth_me(_req: HttpRequest, _app_state: web::Data<AppState>) -> ApiResult {
    todo!()
}
