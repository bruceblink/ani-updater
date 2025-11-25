use crate::domain::po::QueryPage;
use crate::routes::TaskFilter;
use actix_web::web;
use chrono::Utc;
use common::api::{ApiError, PageData};
use serde::Serialize;
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTasksDTO {
    pub id: i64,
    pub name: String,
    pub cron: String,
    pub params: serde_json::Value,
    pub is_enabled: bool,
    pub retry_times: u8,
    pub last_run: Option<chrono::DateTime<Utc>>,
    pub next_run: Option<chrono::DateTime<Utc>>,
    pub last_status: String,
}

#[derive(Debug, FromRow, Clone)]
pub struct ScheduledTasksWithTotal {
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
    pub total_count: i64,
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
            id: task.id,
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

pub async fn list_all_scheduled_tasks_by_page(
    query: web::Query<QueryPage<TaskFilter>>,
    db_pool: &PgPool,
) -> anyhow::Result<PageData<ScheduledTasksDTO>> {
    // 构造带绑定参数的 QueryAs
    let mut result = PageData {
        items: vec![],
        total_count: 0,
        page: 0,
        page_size: 0,
        total_pages: 0,
    };

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"
            SELECT id, name, cron, params, is_enabled, retry_times, last_run, next_run, last_status, created_at, updated_at, COUNT(*) OVER() as total_count
            FROM scheduled_tasks
            WHERE 1 = 1
          "#,
    );

    if let Some(filter) = &query.filter {
        if let Some(name) = &filter.name {
            query_builder.push(" AND name LIKE ");
            query_builder.push_bind(format!("%{name}%"));
        }
        // 查询json字段params 中的arg key
        if let Some(arg) = &filter.arg {
            query_builder.push(" AND params ->> 'arg' LIKE ");
            query_builder.push_bind(arg);
        }
        // 查询json字段params 中的arg key
        if let Some(cmd) = &filter.cmd {
            query_builder.push(" AND params ->> 'cmd' LIKE ");
            query_builder.push_bind(cmd);
        }
    }

    query_builder.push(" ORDER BY updated_at DESC");

    if let Some(page_size) = query.page_size {
        query_builder.push(" LIMIT ");
        query_builder.push_bind(page_size as i64);
        result.page_size = page_size;
    }

    if let (Some(page), Some(page_size)) = (query.page, query.page_size) {
        query_builder.push(" OFFSET ");
        query_builder.push_bind(((page - 1) * page_size) as i64);
        result.page = page;
    }
    // 查询 数据库的原始数据
    let rows: Vec<ScheduledTasksWithTotal> = query_builder
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
            id: task.id,
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

    result.items = data;
    let total_count = if rows.is_empty() {
        0
    } else {
        rows[0].total_count
    };
    result.total_count = total_count as usize;
    let total_pages = if total_count == 0 {
        0
    } else {
        query
            .page_size
            .map(|ps| ((total_count as f64) / (ps as f64)).ceil() as u32)
            .unwrap_or(0)
    };
    result.total_pages = total_pages;
    Ok(result)
}
