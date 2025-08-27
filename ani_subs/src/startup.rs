use crate::routes::get_ani_info::get_ani_info;
use crate::routes::get_ani_info_list;
use crate::routes::health_check::health_check;
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
            .route("/anis/{id}", web::get().to(get_ani_info))
            .route("/anis", web::get().to(get_ani_info_list))
            // 获取连接的副本绑定到应用程序
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
