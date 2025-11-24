use ani_subs::configuration::get_configuration;
use ani_subs::service::initialize_task_manager;
use ani_subs::startup::{create_database_pool, run_database_migrations, start_web_server};
use ani_subs::telemetry::{get_subscriber, init_subscriber};
use anyhow::{Context, Result};
use std::path::PathBuf;

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
