use actix_web::web;
use anyhow::Result;
use chrono::Utc;
use common::NewsFilter;
use common::api::{ApiError, NewsInfo2Item};
use common::dto::NewsItemDTO;
use common::po::{PageData, QueryPage};
use sqlx::{FromRow, PgPool, QueryBuilder};

/// 新闻信息插入或更新
pub async fn upsert_news_item(
    news_item: &NewsInfo2Item,
    tx: &mut sqlx::Transaction<'_, sqlx::postgres::Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
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
        ON CONFLICT (id) DO UPDATE SET
            news_info_id = EXCLUDED.news_info_id,
            title = EXCLUDED.title,
            url = EXCLUDED.url,
            published_at = EXCLUDED.published_at,
            source = EXCLUDED.source,
            content = EXCLUDED.content
        "#,
    )
    .bind(&news_item.news_item_id)
    .bind(news_item.id)
    .bind(&news_item.title)
    .bind(&news_item.url)
    .bind(news_item.news_date)
    .bind(&news_item.name)
    .bind(&news_item.content)
    .execute(tx.as_mut()) // ✅ 正确写法
    .await
    .map_err(|e| {
        tracing::error!("插入或更新 news_item {:?} 失败: {}", news_item, e);
        e
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

/// 分页查询新闻列表
pub async fn list_all_news_item_by_page(
    query: web::Query<QueryPage<NewsFilter>>,
    db_pool: &PgPool,
) -> Result<PageData<NewsItemDTO>> {
    let mut result = PageData {
        items: vec![],
        total_count: 0,
        page: 0,
        page_size: 0,
        total_pages: 0,
    };

    let mut query_builder: QueryBuilder<sqlx::postgres::Postgres> = QueryBuilder::new(
        r#"
            SELECT
                ni.id,
                ni.news_info_id,
                ni.title,
                ni.url,
                ni.published_at,
                ni.source,
                ni.content,
                ni.created_at,
                ni.updated_at,
                COUNT(*) OVER() AS total_count
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

    query_builder.push(" ORDER BY updated_at DESC ");

    // 分页
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

    // 执行查询
    let rows: Vec<NewsItemWithTotal> = query_builder
        .build_query_as()
        .fetch_all(db_pool)
        .await
        .map_err(|e| {
            tracing::error!("数据库查询错误: {e:?}");
            ApiError::Database("数据库查询失败".into())
        })?;

    // 数据转换
    let items = rows
        .iter()
        .map(|item| NewsItemDTO {
            id: item.id.clone(),
            title: item.title.clone(),
            url: item.url.clone(),
            content: item.content.clone(),
            published_at: item.published_at,
            source: item.source,
        })
        .collect();

    result.items = items;

    let total = rows.first().map(|r| r.total_count).unwrap_or(0);
    result.total_count = total as usize;

    result.total_pages = if result.page_size > 0 {
        ((total as f64) / (result.page_size as f64)).ceil() as u32
    } else {
        0
    };

    Ok(result)
}
