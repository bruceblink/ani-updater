use crate::configuration::Setting;
use crate::middleware::{AuthMiddleware, CharsetMiddleware};
use crate::routes::{
    OAuthConfig, SensorData, get_sensor_history, logout, news_get, proxy_image, sse_sensor,
};
use crate::routes::{auth_github_callback, auth_github_login, auth_refresh};
use crate::routes::{get_ani, get_anis};
use crate::routes::{login, me, sync_me_get, sync_me_post, sync_task_source};
use actix_cors::Cors;
use actix_web::dev::Server;
use actix_web::http::header;
use actix_web::{App, HttpServer, web};
use oauth2::basic::BasicClient;
use sqlx::PgPool;
use std::collections::VecDeque;
use std::error::Error;
use std::net::TcpListener;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use tracing_actix_web::TracingLogger;

const MAX_HISTORY: usize = 1000;

/// 初始化app的全局状态变量
#[derive(Clone)]
pub struct AppState {
    history: Arc<RwLock<VecDeque<SensorData>>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_HISTORY))),
        }
    }

    pub async fn add_data(&self, data: SensorData) {
        let mut history = self.history.write().await;
        if history.len() >= MAX_HISTORY {
            history.pop_front();
        }
        history.push_back(data);
    }

    pub async fn get_history(&self) -> Vec<SensorData> {
        let history = self.history.read().await;
        history.iter().cloned().collect()
    }

    pub async fn get_recent(&self, count: usize) -> Vec<SensorData> {
        let history = self.history.read().await;
        let start_idx = history.len().saturating_sub(count);
        history.range(start_idx..).cloned().collect()
    }
}

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    configuration: Setting,
) -> anyhow::Result<Server, Box<dyn Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    // 初始化全局状态变量
    let app_state = web::Data::new(AppState::new());
    let config = OAuthConfig::from_env()?;
    let oauth = BasicClient::new(
        config.client_id,
        Some(config.client_secret),
        config.auth_url,
        Some(config.token_url),
    )
    .set_redirect_uri(config.redirect_url);
    // 智能指针包装一个连接
    let db_pool = web::Data::new(db_pool);
    // 允许的跨域请求的 前端域名白名单列表 FRONTEND_DOMAINS: "http://localhost:3000;http://example.com"
    let allowed_origins: Vec<String> = std::env::var("FRONTEND_DOMAINS")
        .unwrap_or_default()
        .split(';')
        .map(|s| s.to_string())
        .collect();
    info!("允许的跨域请求的前端域名白名单列表: {allowed_origins:?}");
    let allowed_origins = Arc::new(allowed_origins);

    let server = HttpServer::new(move || {
        // clone Arc 引用给闭包
        let cors_allowed_origins = allowed_origins.clone();

        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _req_head| {
                cors_allowed_origins
                    .iter()
                    .any(|o| origin.as_bytes() == o.as_bytes())
            })
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
            .supports_credentials(); // 允许发送 cookie
        App::new()
            .wrap(TracingLogger::default())
            .wrap(cors) // 注册 CORS 中间件
            .wrap(CharsetMiddleware) // 注册字符集中间件
            .app_data(app_state.clone()) // 注册全局状态变量
            .app_data(web::Data::new(configuration.clone())) // 注入全局配置文件
            .app_data(web::Data::new(oauth.clone()))
            .app_data(db_pool.clone())
            .service(auth_github_login)
            .service(auth_github_callback)
            .service(auth_refresh)
            .service(logout)
            .service(sse_sensor)
            .service(get_sensor_history)
            .route("/login", web::post().to(login))
            .service(
                web::scope("/api")
                    .wrap(AuthMiddleware) // 在这里添加需要认证的路由
                    .service(me)
                    .service(sync_task_source)
                    .service(sync_me_get)
                    .service(sync_me_post)
                    .service(proxy_image)
                    .service(news_get)
                    .route("/anis", web::get().to(get_anis))
                    .route("/anis/{id}", web::get().to(get_ani)),
            )
    })
    .listen(listener)?
    .run();
    Ok(server)
}
