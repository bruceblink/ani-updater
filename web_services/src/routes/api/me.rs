use crate::common::ExtractToken;
use actix_web::{HttpRequest, HttpResponse, get};
use common::api::{ApiError, ApiResponse};
use common::po::ApiResult;
use common::utils::verify_jwt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct UserView {
    id: i64,
    username: String,
    roles: Vec<String>,
    permissions: Vec<String>,
}

#[get("/me")]
async fn me(req: HttpRequest) -> ApiResult {
    if let Some(token) = req.get_access_token()
        && let Ok(claims) = verify_jwt(&token)
    {
        let user = UserView {
            id: claims.uid,
            username: claims.sub,
            roles: claims.roles,
            permissions: vec![],
        };
        return Ok(HttpResponse::Ok().json(ApiResponse::ok(user)));
    }
    Err(ApiError::Unauthorized("未携带或非法的 JWT".into()))
}
