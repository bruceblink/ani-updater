use actix_web::body::BoxBody;
use actix_web::{
    Error, HttpMessage, HttpResponse,
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
};
use common::utils::decode_jwt;
use futures::future::{LocalBoxFuture, Ready, ok};
use std::rc::Rc;
// 假设你有 decode_jwt 函数，返回 Result<Claims, Error>

pub struct AuthMiddleware;

impl<S> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Transform = AuthMiddlewareMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddlewareMiddleware {
            service: Rc::new(service),
        })
    }
}

pub struct AuthMiddlewareMiddleware<S> {
    service: Rc<S>,
}

impl<S> Service<ServiceRequest> for AuthMiddlewareMiddleware<S>
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
            let token_opt = req
                .headers()
                .get("Authorization")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer ").map(|s| s.to_string()));

            match token_opt {
                Some(token) => match decode_jwt(&token) {
                    Ok(claims) => {
                        req.extensions_mut().insert(claims);
                        srv.call(req).await
                    }
                    Err(_) => Ok(req.into_response(
                        HttpResponse::Unauthorized()
                            .body("Invalid token")
                            .map_into_boxed_body(),
                    )),
                },
                None => Ok(req.into_response(
                    HttpResponse::Unauthorized()
                        .body("Missing token")
                        .map_into_boxed_body(),
                )),
            }
        })
    }
}
