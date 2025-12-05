use crate::Setting;
use anyhow::Context;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

/// 创建数据库连接池
pub async fn create_database_pool(configuration: &Setting) -> anyhow::Result<PgPool> {
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
pub async fn run_database_migrations(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("../migrations")
        .run(pool)
        .await
        .context("Failed to run database migrations")?;

    Ok(())
}
