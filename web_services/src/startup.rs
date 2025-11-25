use crate::common::AppState;
use crate::configuration::Setting;
use crate::middleware::{AuthMiddleware, CharsetMiddleware};
use crate::routes::{
    OAuthConfig, get_sensor_history, logout, news_get, proxy_image, scheduled_tasks_get,
    sse_sensor, task_reload,
};
use crate::routes::{auth_github_callback, auth_github_login, auth_refresh};
use crate::routes::{get_ani, get_anis};
use crate::routes::{login, me, sync_me_get, sync_me_post, sync_task_source};
use actix_cors::Cors;
use actix_web::dev::Server;
use actix_web::http::header;
use actix_web::{App, HttpServer, web};
use anyhow::{Context, Result};
use oauth2::basic::BasicClient;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use tracing::{info, warn};
use tracing_actix_web::TracingLogger;

pub async fn run(listener: TcpListener, db_pool: PgPool, configuration: Setting) -> Result<Server> {
    // 创建 OAuth 配置和客户端
    let oauth_config = create_oauth_config(configuration.clone())
        .await
        .context("Failed to create OAuth configuration")?;

    let oauth_client =
        create_oauth_client(&oauth_config).context("Failed to create OAuth client")?;

    // 创建应用状态
    let app_state = create_app_state(db_pool, configuration, oauth_config, oauth_client)
        .await
        .context("Failed to create application state")?;

    // 获取允许的源列表
    let allowed_origins = parse_allowed_origins().context("Failed to parse allowed origins")?;

    // 创建并启动服务器
    create_server(listener, app_state, allowed_origins)
        .await
        .context("Failed to create server")
}

/// 创建 OAuth 配置
async fn create_oauth_config(configuration: Setting) -> Result<OAuthConfig> {
    OAuthConfig::from_configuration(configuration)
        .context("Failed to load OAuth configuration from environment variables")
}

/// 创建 OAuth 客户端
fn create_oauth_client(config: &OAuthConfig) -> Result<BasicClient> {
    let client = BasicClient::new(
        config.client_id.clone(),
        Some(config.client_secret.clone()),
        config.auth_url.clone(),
        Some(config.token_url.clone()),
    )
    .set_redirect_uri(config.redirect_url.clone());

    Ok(client)
}

/// 创建应用状态
async fn create_app_state(
    db_pool: PgPool,
    configuration: Setting,
    oauth_config: OAuthConfig,
    oauth_client: BasicClient,
) -> Result<web::Data<AppState>> {
    let app_state = AppState::create_app_state(db_pool, configuration, oauth_config, oauth_client)
        .await
        .context("Failed to initialize application state")?;

    Ok(web::Data::new(app_state))
}

/// 解析允许的源域名
fn parse_allowed_origins() -> Result<Vec<String>> {
    let env_value = std::env::var("FRONTEND_DOMAINS").unwrap_or_default();

    let origins: Vec<String> = env_value
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    // 记录日志
    match (env_value.is_empty(), origins.is_empty()) {
        (true, _) => info!("FRONTEND_DOMAINS not set, CORS will only allow same-origin requests"),
        (false, true) => {
            warn!("FRONTEND_DOMAINS is set but contains no valid origins after parsing")
        }
        (false, false) => info!("CORS allowed origins: {:?}", origins),
    }

    Ok(origins)
}

/// 配置 CORS 中间件
fn configure_cors(allowed_origins: Vec<String>) -> Cors {
    if allowed_origins.is_empty() {
        // 如果没有配置允许的源，使用默认设置（仅允许同源）
        Cors::default()
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
            .supports_credentials()
    } else {
        // 根据配置的源列表设置 CORS
        // 将 allowed_origins 移动到闭包中
        Cors::default()
            .allowed_origin_fn(move |origin, _req_head| {
                allowed_origins
                    .iter()
                    .any(|o| origin.as_bytes() == o.as_bytes())
            })
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
            .supports_credentials()
    }
}

/// 创建服务器
async fn create_server(
    listener: TcpListener,
    app_state: web::Data<AppState>,
    allowed_origins: Vec<String>,
) -> Result<Server> {
    let server = HttpServer::new(move || {
        // 在闭包内部创建 CORS 中间件，传递所有权
        let cors = configure_cors(allowed_origins.clone());

        App::new()
            .wrap(TracingLogger::default())
            .wrap(cors) // 注册 CORS 中间件
            .wrap(CharsetMiddleware) // 注册字符集中间件
            .app_data(app_state.clone()) // 注册全局状态
            // 公开路由（无需认证）
            .service(auth_github_login)
            .service(auth_github_callback)
            .service(auth_refresh)
            .service(logout)
            .service(sse_sensor)
            .service(get_sensor_history)
            .route("/login", web::post().to(login))
            // 需要认证的 API 路由
            .service(
                web::scope("/api")
                    .wrap(AuthMiddleware)
                    .service(me)
                    .service(sync_task_source)
                    .service(sync_me_get)
                    .service(sync_me_post)
                    .service(proxy_image)
                    .service(news_get)
                    .service(scheduled_tasks_get)
                    .route("/anis", web::get().to(get_anis))
                    .route("/anis/{id}", web::get().to(get_ani)),
            )
            // 管理员路由
            .service(
                web::scope("/admin")
                    .wrap(AuthMiddleware)
                    .service(task_reload),
            )
    })
    .listen(listener)
    .context("Failed to bind to listener")?
    .run();

    Ok(server)
}

/// 创建数据库连接池
pub async fn create_database_pool(configuration: &Setting) -> Result<PgPool> {
    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        return PgPoolOptions::new()
            .max_connections(configuration.database.max_connections)
            .connect(&database_url)
            .await
            .context("Failed to connect to database using DATABASE_URL environment variable");
    }

    // 只有在没有环境变量时，才需要克隆 configuration.database
    let database_settings = configuration.database.clone();
    let connect_options = database_settings.connect_options();

    PgPoolOptions::new()
        .max_connections(database_settings.max_connections)
        .connect_with(connect_options)
        .await
        .context("Failed to connect to database using configuration settings")
}

/// 运行数据库迁移
pub async fn run_database_migrations(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("../migrations")
        .run(pool)
        .await
        .context("Failed to run database migrations")?;

    Ok(())
}

/// 启动 Web 服务器
pub async fn start_web_server(configuration: Setting, connection_pool: PgPool) -> Result<()> {
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );

    let listener = TcpListener::bind(&address)
        .with_context(|| format!("Failed to bind to address: {}", address))?;

    let server = run(listener, connection_pool, configuration)
        .await
        .context("Failed to start server")?;

    server.await.context("Server error during execution")
}
