use crate::common::AppState;
use actix_web::{HttpRequest, post, web};
use common::po::ApiResult;

/// 签发 Service Token的API <br>
/// /auth/service/token  POST请求
#[post("/auth/service/token")]
async fn auth_service_token(_req: HttpRequest, _app_state: web::Data<AppState>) -> ApiResult {
    todo!()
}

/// 吊销 Service Token的API <br>
/// /auth/service/revoke  POST请求
#[post("/auth/service/revoke")]
async fn auth_service_revoke(_req: HttpRequest, _app_state: web::Data<AppState>) -> ApiResult {
    todo!()
}
