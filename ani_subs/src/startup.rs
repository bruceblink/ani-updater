use crate::routes::get_ani;
use crate::routes::get_anis;
use crate::routes::health_check;
use crate::routes::login;
use actix_web::dev::Server;
use actix_web::{App, HttpServer, web};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    // 智能指针包装一个连接
    let db_pool = web::Data::new(db_pool);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/anis/{id}", web::get().to(get_ani))
            .route("/anis", web::get().to(get_anis))
            .route("/login", web::post().to(login))
            // 获取连接的副本绑定到应用程序
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
