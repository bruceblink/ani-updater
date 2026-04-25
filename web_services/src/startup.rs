use crate::common::AppState;
use crate::middleware::{AuthMiddleware, CharsetMiddleware};
use crate::routes::register::register;
use crate::routes::{
    ani_collect_create, ani_collect_delete, ani_collect_list, ani_collect_watched, health,
};
use crate::routes::{auth_github_callback, auth_github_login, auth_token_refresh};
use crate::routes::{get_ani, get_anis};
use crate::routes::{
    logout, news_event_items_get, news_events_get, news_get, news_items_get, news_stream_sse,
    proxy_image, scheduled_tasks_create, scheduled_tasks_delete, scheduled_tasks_get,
    scheduled_tasks_toggle, scheduled_tasks_update, task_reload,
};
use crate::routes::{me, sync_me_get, sync_me_post, sync_task_source};
use actix_web::dev::Server;
use actix_web::{App, HttpServer, web};
use anyhow::{Context, Result};
use infra::{OAuthConfig, Setting, configure_cors, create_oauth_client, try_create_oauth_config};
use oauth2::basic::BasicClient;
use sqlx::PgPool;
use std::net::TcpListener;
use tracing::{info, warn};
use tracing_actix_web::TracingLogger;

pub async fn run(listener: TcpListener, db_pool: PgPool, configuration: Setting) -> Result<Server> {
    // 尝试创建 OAuth 配置（未配置时返回 None，不影响启动）
    let oauth_config = try_create_oauth_config(&configuration).context("OAuth 配置解析失败")?;

    let oauth_client = oauth_config
        .as_ref()
        .map(|cfg| create_oauth_client(cfg).context("Failed to create OAuth client"))
        .transpose()?;

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

/// 创建应用状态
async fn create_app_state(
    db_pool: PgPool,
    configuration: Setting,
    oauth_config: Option<OAuthConfig>,
    oauth_client: Option<BasicClient>,
) -> Result<web::Data<AppState>> {
    let app_state = AppState::create_app_state(db_pool, configuration, oauth_config, oauth_client)
        .await
        .context("Failed to initialize application state")?;

    Ok(web::Data::new(app_state))
}

/// 创建服务器
async fn create_server(
    listener: TcpListener,
    app_state: web::Data<AppState>,
    allowed_origins: Vec<String>,
) -> Result<Server> {
    // 仅在 OAuth 已配置时注册 GitHub 登录路由
    let has_oauth = app_state.oauth_config.is_some();

    let server = HttpServer::new(move || {
        // 在闭包内部创建 CORS 中间件，传递所有权
        let cors = configure_cors(allowed_origins.clone());

        let mut app = App::new()
            .wrap(TracingLogger::default())
            .wrap(cors)
            .wrap(CharsetMiddleware)
            .app_data(app_state.clone())
            // 公开路由（无需认证）
            .service(health)
            .service(logout)
            .service(register)
            .service(auth_token_refresh)
            // SSE 公开接口（无需认证，供落地页实时新闻使用）
            // 需要认证的 API 路由
            .service(
                web::scope("/api").service(news_stream_sse).service(
                    web::scope("")
                        .wrap(AuthMiddleware)
                        .service(me)
                        .service(sync_task_source)
                        .service(sync_me_get)
                        .service(sync_me_post)
                        .service(proxy_image)
                        .service(news_get)
                        .service(scheduled_tasks_get)
                        .service(scheduled_tasks_create)
                        .service(scheduled_tasks_update)
                        .service(scheduled_tasks_toggle)
                        .service(scheduled_tasks_delete)
                        .service(ani_collect_list)
                        .service(ani_collect_create)
                        .service(ani_collect_delete)
                        .service(ani_collect_watched)
                        .service(news_items_get)
                        .service(news_events_get)
                        .service(news_event_items_get)
                        .route("/anis", web::get().to(get_anis))
                        .route("/anis/{id}", web::get().to(get_ani)),
                ),
            )
            .service(
                web::scope("/admin")
                    .wrap(AuthMiddleware)
                    .service(task_reload),
            );

        // 仅当 GitHub OAuth 已配置时注册对应路由
        if has_oauth {
            app = app.service(auth_github_login).service(auth_github_callback);
        }

        app
    })
    .listen(listener)
    .context("Failed to bind to listener")?
    .run();

    Ok(server)
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
