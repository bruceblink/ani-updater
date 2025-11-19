use crate::domain::po::QueryPage;
use crate::routes::{NewsFilter, NewsInfoDTO};
use actix_web::web;
use anyhow::Result;
use chrono::Utc;
use common::api::{ApiError, NewsItem, PageData};
use serde_json::json;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

/// 新闻信息插入新记录
pub async fn upsert_news_info(news_item: &NewsItem, db_pool: &PgPool) -> Result<()> {
    let data = json!({
        "items": news_item.items
    });
    let _ = sqlx::query(
        r#"
        INSERT INTO public.news_info (
            news_from,
            data
        ) VALUES ($1, $2)
        ON CONFLICT (news_from, news_date) DO UPDATE SET
            news_from = EXCLUDED.news_from,
            data = EXCLUDED.data
        "#,
    )
    .bind(&news_item.id)
    .bind(data)
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("插入或更新 news_info {:?} 失败: {}", news_item, e);
        anyhow::anyhow!(e)
    })?;

    Ok(())
}

#[derive(Debug, FromRow, Clone)]
struct NewsInfoWithTotal {
    #[allow(dead_code)]
    pub id: i64,
    pub news_from: String,
    pub news_date: chrono::NaiveDate,
    pub data: serde_json::Value,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: Option<chrono::DateTime<Utc>>,
    pub total_count: i64,
}

pub async fn list_all_news_info_by_page(
    query: web::Query<QueryPage<NewsFilter>>,
    db_pool: &PgPool,
) -> Result<PageData<NewsInfoDTO>> {
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
            SELECT ni.id, ni.news_from, ni.news_date, ni.data, ni.created_at, ni.updated_at, COUNT(*) OVER() as total_count
            FROM news_info ni
            WHERE 1 = 1
          "#,
    );

    if let Some(filter) = &query.filter {
        if let Some(news_from) = &filter.news_from {
            query_builder.push(" AND news_from LIKE ");
            query_builder.push_bind(format!("%{news_from}%"));
        }
        if let Some(news_date) = &filter.news_date {
            query_builder.push(" AND news_date = ");
            query_builder.push_bind(news_date);
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
    let rows: Vec<NewsInfoWithTotal> = query_builder
        .build_query_as()
        .fetch_all(db_pool)
        .await
        .map_err(|e| {
            tracing::error!("数据库查询错误: {e:?}");
            ApiError::Database("数据库查询失败".into())
        })?;
    // 转换数据库数据为前端需要的的DTO数据
    let data: Vec<NewsInfoDTO> = rows
        .iter()
        .map(|news_info| NewsInfoDTO {
            id: news_info.id,
            news_from: news_info.news_from.clone(),
            news_date: news_info.news_date,
            data: news_info.data.clone(),
            created_at: news_info.created_at,
            updated_at: news_info.updated_at,
        })
        .collect::<Vec<NewsInfoDTO>>();

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
