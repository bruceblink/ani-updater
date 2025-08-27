use tracing_actix_web::root_span_macro::private::tracing::Subscriber;
use tracing_actix_web::root_span_macro::private::tracing::dispatcher::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry, fmt};

/// 构建 Subscriber（根据环境决定使用 Pretty 还是 JSON 格式化）
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> Box<dyn Subscriber + Send + Sync>
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));

    // 1. 编译模式默认：debug -> local, release -> production
    #[cfg(debug_assertions)]
    let mut is_local = true;
    #[cfg(not(debug_assertions))]
    let mut is_local = false;

    // 2. 环境变量覆盖（APP_ENV=local 强制本地彩色日志）
    if let Ok(env) = std::env::var("APP_ENV") {
        if env.to_lowercase() == "local" {
            is_local = true;
        }
    }

    if is_local {
        // 本地：彩色 + compact
        let formatting_layer = fmt::layer().with_writer(sink).with_target(true).compact();
        //.pretty();
        Box::new(Registry::default().with(env_filter).with(formatting_layer))
    } else {
        // 生产：JSON 结构化日志（bunyan）
        let formatting_layer = BunyanFormattingLayer::new(name, sink);
        Box::new(
            Registry::default()
                .with(env_filter)
                .with(JsonStorageLayer)
                .with(formatting_layer),
        )
    }
}

/// 全局初始化 Subscriber
pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber.into()).expect("Failed to set subscriber");
}
