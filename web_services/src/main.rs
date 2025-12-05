use anyhow::{Context, Result};
use infra::{
    create_database_pool, get_configuration, get_subscriber, init_subscriber,
    run_database_migrations,
};
use std::path::PathBuf;
use web_services::service::initialize_task_manager;
use web_services::startup::start_web_server;

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
