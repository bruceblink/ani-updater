use crate::middleware::{AuthMiddleware, CharsetMiddleware};
use crate::routes::OAuthConfig;
use crate::routes::get_ani;
use crate::routes::get_anis;
use crate::routes::health_check;
use crate::routes::login;
use crate::routes::{github_callback, github_login, index, me};
use actix_web::dev::Server;
use actix_web::{App, HttpServer, web};
use oauth2::basic::BasicClient;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    dotenvy::dotenv().ok();

    let config = OAuthConfig::from_env();
    let oauth = BasicClient::new(
        config.client_id,
        Some(config.client_secret),
        config.auth_url,
        Some(config.token_url),
    )
    .set_redirect_uri(config.redirect_url);
    // 智能指针包装一个连接
    let db_pool = web::Data::new(db_pool);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(CharsetMiddleware)
            .app_data(web::Data::new(oauth.clone()))
            .app_data(db_pool.clone())
            .service(index)
            .service(github_login)
            .service(github_callback)
            .route("/login", web::post().to(login))
            .route("/health_check", web::get().to(health_check))
            // 获取连接的副本绑定到应用程序
            .service(
                web::scope("/api")
                    .wrap(AuthMiddleware) // 在这里添加需要认证的路由
                    .service(me)
                    .route("/anis", web::get().to(get_anis))
                    .route("/anis/{id}", web::get().to(get_ani)),
            )
    })
    .listen(listener)?
    .run();
    Ok(server)
}
