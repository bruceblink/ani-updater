use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use std::path::PathBuf;

use ani_subs::configuration::get_configuration;
use ani_subs::startup::run;
use ani_subs::telemetry::{get_subscriber, init_subscriber};
use ani_subs::timer_tasker::start_async_timer_task;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    //初始化日志组件
    let subscriber = get_subscriber("ani-updater".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);
    let configuration = get_configuration(Some(PathBuf::from("./configuration")), None)
        .expect("Failed to read configuration.");
    let connection_pool =
        PgPoolOptions::new().connect_lazy_with(configuration.database.connect_options());
    // 启动异步定时任务
    let task_config = configuration.datasource;
    start_async_timer_task(task_config["anime"].clone()).await;
    let address = format!(
        "{}:{}",
        configuration.application.host, configuration.application.port
    );
    let listener = TcpListener::bind(address).expect("Failed to bind random port");
    run(listener, connection_pool)?.await?;
    Ok(())
}
