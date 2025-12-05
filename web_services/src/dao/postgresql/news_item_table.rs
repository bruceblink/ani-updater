use crate::routes::NewsFilter;
use actix_web::web;
use anyhow::Result;
use chrono::Utc;
use common::api::{ApiError, NewsInfo2Item};
use common::po::{PageData, QueryPage};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

pub struct NewsItemDTO {
    pub id: String,
    pub title: String,
    pub url: String,
    pub content: serde_json::Value,
    pub source: chrono::DateTime<Utc>,
    pub published_at: Option<chrono::DateTime<Utc>>,
}

/// 新闻信息插入新记录
pub async fn upsert_news_item(news_item: &NewsInfo2Item, db_pool: &PgPool) -> Result<()> {
    let _ = sqlx::query(
        r#"
        INSERT INTO public.news_item (
            id,
            news_info_id,
            title,
            url,
            published_at,
            source,
            content
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(&news_item.news_item_id)
    .bind(&news_item.id)
    .bind(&news_item.title)
    .bind(&news_item.url)
    .bind(news_item.news_date)
    .bind(&news_item.news_from)
    .bind(&news_item.content)
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("插入或更新 news_item {:?} 失败: {}", news_item, e);
        anyhow::anyhow!(e)
    })?;

    Ok(())
}

#[derive(Debug, FromRow, Clone)]
struct NewsItemWithTotal {
    pub id: String,
    #[allow(dead_code)]
    pub news_info_id: String,
    pub title: String,
    pub url: String,
    pub content: serde_json::Value,
    pub source: chrono::DateTime<Utc>,
    pub published_at: Option<chrono::DateTime<Utc>>,
    #[allow(dead_code)]
    pub created_at: Option<chrono::DateTime<Utc>>,
    #[allow(dead_code)]
    pub updated_at: Option<chrono::DateTime<Utc>>,
    pub total_count: i64,
}

pub async fn list_all_news_item_by_page(
    query: web::Query<QueryPage<NewsFilter>>,
    db_pool: &PgPool,
) -> Result<PageData<NewsItemDTO>> {
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
            SELECT ni.id, ni.news_info_id, ni.title, ni.url, ni.published_at, ni.source, ni.content, ni.created_at, ni.updated_at COUNT(*) OVER() as total_count
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
    let rows: Vec<NewsItemWithTotal> = query_builder
        .build_query_as()
        .fetch_all(db_pool)
        .await
        .map_err(|e| {
            tracing::error!("数据库查询错误: {e:?}");
            ApiError::Database("数据库查询失败".into())
        })?;
    // 转换数据库数据为前端需要的的DTO数据
    let data: Vec<NewsItemDTO> = rows
        .iter()
        .map(|news_info| NewsItemDTO {
            id: news_info.id.clone(),
            title: news_info.title.clone(),
            url: news_info.url.clone(),
            content: news_info.content.clone(),
            published_at: news_info.published_at,
            source: news_info.source,
        })
        .collect::<Vec<NewsItemDTO>>();

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
