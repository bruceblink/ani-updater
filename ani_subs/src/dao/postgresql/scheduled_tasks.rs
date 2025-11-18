use chrono::Utc;
use common::api::ApiError;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

#[derive(Debug, FromRow, Clone)]
pub struct ScheduledTasks {
    #[allow(dead_code)]
    pub id: i64,
    pub name: String,
    pub cron: String,
    pub params: serde_json::Value,
    pub is_enabled: bool,
    pub retry_times: i16,
    pub last_run: Option<chrono::DateTime<Utc>>,
    pub next_run: Option<chrono::DateTime<Utc>>,
    pub last_status: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: Option<chrono::DateTime<Utc>>,
}

pub struct ScheduledTasksDTO {
    pub name: String,
    pub cron: String,
    pub params: serde_json::Value,
    pub is_enabled: bool,
    pub retry_times: u8,
    pub last_run: Option<chrono::DateTime<Utc>>,
    pub next_run: Option<chrono::DateTime<Utc>>,
    pub last_status: String,
}

pub async fn list_all_scheduled_tasks(db_pool: &PgPool) -> anyhow::Result<Vec<ScheduledTasksDTO>> {
    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
            SELECT id, name, cron, params, is_enabled, retry_times, last_run, next_run, last_status, created_at, updated_at
            FROM scheduled_tasks
          "#,
    );

    // 查询 数据库的原始数据
    let rows: Vec<ScheduledTasks> = query_builder
        .build_query_as()
        .fetch_all(db_pool)
        .await
        .map_err(|e| {
            tracing::error!("数据库查询错误: {e:?}");
            ApiError::Database("数据库查询失败".into())
        })?;
    // 转换数据库数据为前端需要的的DTO数据
    let data: Vec<ScheduledTasksDTO> = rows
        .iter()
        .map(|task| ScheduledTasksDTO {
            name: task.name.clone(),
            cron: task.cron.clone(),
            params: task.params.clone(),
            is_enabled: task.is_enabled,
            retry_times: task.retry_times as u8,
            last_run: task.last_run,
            next_run: task.next_run,
            last_status: task.last_status.clone(),
        })
        .collect::<Vec<ScheduledTasksDTO>>();
    Ok(data)
}
