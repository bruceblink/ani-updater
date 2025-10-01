use crate::common::ExtractToken;
use crate::configuration::Setting;
use actix_web::{HttpRequest, HttpResponse, get, web};
use common::api::{ApiError, ApiResponse, ApiResult};
use common::utils::verify_jwt;
use sqlx::PgPool;

#[get("/sync/me")]
async fn sync_me_get(
    req: HttpRequest,
    _db: web::Data<PgPool>,
    _config: web::Data<Setting>,
) -> ApiResult {
    if let Some(token) = req.get_access_token()
        && let Ok(claims) = verify_jwt(&token)
    {
        return Ok(HttpResponse::Ok().json(ApiResponse::ok(claims)));
    }
    Err(ApiError::Unauthorized("未携带或非法的 JWT".into()))
}
