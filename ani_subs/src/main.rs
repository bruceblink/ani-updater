use ani_subs::configuration::{DatabaseSettings, Setting, get_configuration};
use ani_subs::service::initialize_task_manager;
use ani_subs::startup::run;
use ani_subs::telemetry::{get_subscriber, init_subscriber};
use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use std::path::PathBuf;

// 设置默认最大连接数
const DEFAULT_MAX_CONNECTIONS: u32 = 10;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志组件
    let subscriber = get_subscriber("ani-updater".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    // 读取配置文件
    let configuration = get_configuration(Some(PathBuf::from("./configuration")))
        .context("Failed to read configuration")?;

    // 创建数据库连接池
    let connection_pool = create_database_pool(&configuration)
        .await
        .context("Failed to create database connection pool")?;

    // 运行数据库迁移
    run_database_migrations(&connection_pool)
        .await
        .context("Failed to run database migrations")?;

    // 初始化定时任务
    initialize_task_manager(connection_pool.clone())
        .await
        .context("Failed to initialize task manager")?;

    // 启动 web 服务
    start_web_server(configuration, connection_pool)
        .await
        .context("Failed to start web server")?;

    Ok(())
}

/// 创建数据库连接池
async fn create_database_pool(configuration: &Setting) -> Result<sqlx::PgPool> {
    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        return PgPoolOptions::new()
            .max_connections(DEFAULT_MAX_CONNECTIONS)
            .connect(&database_url)
            .await
            .context("Failed to connect to database using DATABASE_URL environment variable");
    }

    // 只有在没有环境变量时，才需要克隆 configuration.database
    let database_settings = DatabaseSettings::from_env(configuration.database.clone());
    let connect_options = database_settings.connect_options();

    PgPoolOptions::new()
        .max_connections(DEFAULT_MAX_CONNECTIONS)
        .connect_with(connect_options)
        .await
        .context("Failed to connect to database using configuration settings")
}

/// 运行数据库迁移
async fn run_database_migrations(pool: &sqlx::PgPool) -> Result<()> {
    sqlx::migrate!("../migrations")
        .run(pool)
        .await
        .context("Failed to run database migrations")?;

    Ok(())
}

/// 启动 Web 服务器
async fn start_web_server(configuration: Setting, connection_pool: sqlx::PgPool) -> Result<()> {
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
