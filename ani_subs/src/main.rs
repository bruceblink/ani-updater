use ani_subs::service::task_service::start_async_timer_task;
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use std::path::PathBuf;

use ani_subs::configuration::{DatabaseSettings, get_configuration};
use ani_subs::startup::run;
use ani_subs::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    //初始化日志组件
    let subscriber = get_subscriber("ani-updater".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    // 读取配置文件
    let configuration = get_configuration(Some(PathBuf::from("./configuration")))
        .expect("Failed to read configuration.");
    // 创建数据库连接池
    let connection_pool = PgPoolOptions::new()
        .connect_lazy_with(DatabaseSettings::from_env(configuration.database).connect_options());
    // 运行数据库迁移
    sqlx::migrate!("../migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    // 启动异步定时任务
    let task_config = configuration.task_config;
    start_async_timer_task(task_config["anime"].clone(), connection_pool.clone()).await;
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address).expect("Failed to bind random port");
    // 启动web服务
    run(listener, connection_pool)?.await?;
    Ok(())
}
