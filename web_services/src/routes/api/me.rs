use actix_web::{HttpMessage, HttpRequest, HttpResponse, get};
use common::api::{ApiError, ApiResponse};
use common::po::ApiResult;
use common::utils::JwtClaims;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct UserView {
    id: i64,
    username: String,
    email: Option<String>,
    avatar: Option<String>,
    roles: Vec<String>,
    permissions: Vec<String>,
}

#[get("/me")]
async fn me(req: HttpRequest) -> ApiResult {
    let claims = req
        .extensions()
        .get::<JwtClaims>()
        .cloned()
        .ok_or_else(|| ApiError::Unauthorized("未携带或非法的 JWT".into()))?;

    let user = UserView {
        id: claims.uid,
        username: claims.sub,
        email: claims.email,
        avatar: claims.avatar,
        roles: claims.roles,
        permissions: vec![],
    };
    Ok(HttpResponse::Ok().json(ApiResponse::ok(user)))
}
