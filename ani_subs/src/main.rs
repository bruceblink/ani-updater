use ani_subs::configuration::{DatabaseSettings, get_configuration};
use ani_subs::service::initialize_task_manager;
use ani_subs::startup::run;
use ani_subs::telemetry::{get_subscriber, init_subscriber};
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use std::path::PathBuf;

// 设置默认最大连接数
const DEFAULT_MAX_CONNECTIONS: u32 = 10;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //初始化日志组件
    let subscriber = get_subscriber("ani-updater".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    // 读取配置文件
    let configuration = get_configuration(Some(PathBuf::from("./configuration")))
        .expect("Failed to read configuration.");
    // 创建数据库连接池
    let connection_pool = if let Ok(database_url) = std::env::var("DATABASE_URL") {
        // 优先使用环境变量中的完整连接字符串
        PgPoolOptions::new()
            .max_connections(DEFAULT_MAX_CONNECTIONS)
            .connect_lazy(&database_url)
            .expect("Failed to create pool from DATABASE_URL")
    } else {
        // 回退到原来的配置方式
        PgPoolOptions::new()
            .max_connections(DEFAULT_MAX_CONNECTIONS)
            .connect_lazy_with(
                DatabaseSettings::from_env(configuration.clone().database).connect_options(),
            )
    };
    // 运行数据库迁移
    sqlx::migrate!("../migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    // 初始化定时任务
    initialize_task_manager(connection_pool.clone()).await?;
    let address = format!(
        "{}:{}",
        configuration.clone().application.host,
        configuration.clone().application.port
    );
    let listener = TcpListener::bind(address).expect("Failed to bind random port");
    // 启动 web 服务
    let server = run(listener, connection_pool, configuration).await?; // run 返回 Result<Server, Box<dyn Error>>
    server.await?;
    Ok(())
}
