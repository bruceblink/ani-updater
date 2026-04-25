use actix_web::{HttpResponse, get};
use common::api::ApiResponse;
use common::po::ApiResult;
use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[get("/")]
async fn index() -> ApiResult {
    Ok(HttpResponse::Ok().json(ApiResponse::ok("使用 GitHub 进行第三方登录")))
}

#[get("/health")]
async fn health() -> ApiResult {
    Ok(HttpResponse::Ok().json(ApiResponse::ok(HealthResponse { status: "ok" })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, body::to_bytes, http::StatusCode, test};
    use serde_json::Value;

    #[actix_web::test]
    async fn health_returns_ok_status() {
        let app = test::init_service(App::new().service(health)).await;
        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);

        let body = to_bytes(resp.into_body()).await.unwrap();
        let data: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(data["status"], "ok");
        assert_eq!(data["data"]["status"], "ok");
    }
}
