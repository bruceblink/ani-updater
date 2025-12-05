use crate::common::ExtractToken;
use actix_web::{HttpRequest, HttpResponse, get};
use common::api::{ApiError, ApiResponse};
use common::po::ApiResult;
use common::utils::verify_jwt;

#[get("/me")]
async fn me(req: HttpRequest) -> ApiResult {
    if let Some(token) = req.get_access_token()
        && let Ok(claims) = verify_jwt(&token)
    {
        return Ok(HttpResponse::Ok().json(ApiResponse::ok(claims)));
    }
    Err(ApiError::Unauthorized("未携带或非法的 JWT".into()))
}
