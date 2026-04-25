use crate::common::ExtractToken;
use actix_web::body::BoxBody;
use actix_web::{
    Error, HttpMessage, HttpResponse,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
};
use common::api::ApiResponse;
use common::utils::verify_jwt;
use futures::future::{LocalBoxFuture, Ready, ok};
use std::rc::Rc;

pub struct AuthMiddleware;

impl<S> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = AuthMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddlewareService {
            service: Rc::new(service),
        })
    }
}

pub struct AuthMiddlewareService<S> {
    service: Rc<S>,
}

impl<S> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &self,
        ctx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let srv = Rc::clone(&self.service);

        Box::pin(async move {
            let token_opt = req.get_access_token();
            match token_opt {
                Some(token) => match verify_jwt(&token) {
                    Ok(claims) => {
                        req.extensions_mut().insert(claims);
                        srv.call(req).await
                    }
                    Err(_) => Ok(req.into_response(
                        HttpResponse::Unauthorized()
                            .json(ApiResponse::<()>::err("Invalid token"))
                            .map_into_boxed_body(),
                    )),
                },
                None => Ok(req.into_response(
                    HttpResponse::Unauthorized()
                        .json(ApiResponse::<()>::err("Missing token"))
                        .map_into_boxed_body(),
                )),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, HttpResponse, body::to_bytes, http::StatusCode, test, web};
    use serde_json::Value;

    async fn protected() -> HttpResponse {
        HttpResponse::Ok().finish()
    }

    #[actix_web::test]
    async fn auth_middleware_rejects_missing_token() {
        let app = test::init_service(
            App::new().service(
                web::scope("/api")
                    .wrap(AuthMiddleware)
                    .route("/protected", web::get().to(protected)),
            ),
        )
        .await;

        let req = test::TestRequest::get().uri("/api/protected").to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let body = to_bytes(resp.into_body()).await.unwrap();
        let data: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(data["status"], "error");
        assert_eq!(data["message"], "Missing token");
    }
}
