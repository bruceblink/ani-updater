use actix_web::web;
use anyhow::Result;
use common::api::{ApiError, NewsInfo2Item};
use common::dto::{NewsEventDTO, NewsItemResponseDTO};
use common::po::{PageData, QueryPage};
use common::{NewsEventFilter, NewsItemFilter};
use sqlx::{PgPool, QueryBuilder};

/// 新闻信息插入或更新（内部任务使用）
pub async fn upsert_news_item(
    news_item: &NewsInfo2Item,
    tx: &mut sqlx::Transaction<'_, sqlx::postgres::Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO news_item (
            item_id,
            news_info_id,
            title,
            url,
            published_at,
            source,
            content
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (item_id, published_at) DO UPDATE SET
            news_info_id = EXCLUDED.news_info_id,
            title = EXCLUDED.title,
            url = EXCLUDED.url,
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
    .execute(tx.as_mut())
    .await
    .map_err(|e| {
        tracing::error!("插入或更新 news_item {:?} 失败: {}", news_item, e);
        e
    })?;

    Ok(())
}

/// 分页查询新闻条目列表
pub async fn list_all_news_item_by_page(
    query: web::Query<QueryPage<NewsItemFilter>>,
    db_pool: &PgPool,
) -> Result<PageData<NewsItemResponseDTO>> {
    let mut result = PageData {
        items: vec![],
        total_count: 0,
        page: 1,
        page_size: 20,
        total_pages: 0,
    };

    #[derive(Debug, sqlx::FromRow)]
    struct Row {
        pub id: i64,
        pub item_id: String,
        pub title: String,
        pub url: String,
        pub source: Option<String>,
        pub published_at: chrono::NaiveDate,
        pub cluster_id: Option<i64>,
        pub extracted: bool,
        pub created_at: Option<chrono::DateTime<chrono::Utc>>,
        pub total_count: i64,
    }

    let mut qb: QueryBuilder<sqlx::postgres::Postgres> = QueryBuilder::new(
        r#"
            SELECT
                id,
                item_id,
                title,
                url,
                source,
                published_at,
                cluster_id,
                extracted,
                created_at,
                COUNT(*) OVER() AS total_count
            FROM news_item
            WHERE 1 = 1
        "#,
    );

    if let Some(filter) = &query.filter {
        if let Some(source) = &filter.source {
            qb.push(" AND source LIKE ");
            qb.push_bind(format!("%{source}%"));
        }
        if let Some(published_at) = &filter.published_at {
            qb.push(" AND published_at = ");
            qb.push_bind(published_at.as_str());
        }
        if let Some(cluster_id) = filter.cluster_id {
            qb.push(" AND cluster_id = ");
            qb.push_bind(cluster_id);
        }
        if let Some(extracted) = filter.extracted {
            qb.push(" AND extracted = ");
            qb.push_bind(extracted);
        }
    }

    qb.push(" ORDER BY published_at DESC, created_at DESC ");

    let page_size = query.page_size.unwrap_or(20);
    let page = query.page.unwrap_or(1).max(1);

    qb.push(" LIMIT ");
    qb.push_bind(page_size as i64);
    qb.push(" OFFSET ");
    qb.push_bind(((page - 1) * page_size) as i64);

    result.page = page;
    result.page_size = page_size;

    let rows: Vec<Row> = qb.build_query_as().fetch_all(db_pool).await.map_err(|e| {
        tracing::error!("查询新闻条目失败: {e:?}");
        ApiError::Database("数据库查询失败".into())
    })?;

    let total = rows.first().map(|r| r.total_count).unwrap_or(0);
    result.total_count = total as usize;
    result.total_pages = if page_size > 0 {
        ((total as f64) / (page_size as f64)).ceil() as u32
    } else {
        0
    };
    result.items = rows
        .into_iter()
        .map(|r| NewsItemResponseDTO {
            id: r.id,
            item_id: r.item_id,
            title: r.title,
            url: r.url,
            source: r.source,
            published_at: r.published_at,
            cluster_id: r.cluster_id,
            extracted: r.extracted,
            created_at: r.created_at,
        })
        .collect();

    Ok(result)
}

/// 分页查询新闻热点事件列表
pub async fn list_all_news_event_by_page(
    query: web::Query<QueryPage<NewsEventFilter>>,
    db_pool: &PgPool,
) -> Result<PageData<NewsEventDTO>> {
    let mut result = PageData {
        items: vec![],
        total_count: 0,
        page: 1,
        page_size: 20,
        total_pages: 0,
    };

    #[derive(Debug, sqlx::FromRow)]
    struct Row {
        pub id: i64,
        pub event_date: chrono::NaiveDate,
        pub cluster_id: i64,
        pub title: Option<String>,
        pub summary: Option<String>,
        pub news_count: i32,
        pub score: Option<f32>,
        pub status: i16,
        pub parent_event_id: Option<i64>,
        pub created_at: chrono::DateTime<chrono::Utc>,
        pub total_count: i64,
    }

    let mut qb: QueryBuilder<sqlx::postgres::Postgres> = QueryBuilder::new(
        r#"
            SELECT
                id,
                event_date,
                cluster_id,
                title,
                summary,
                news_count,
                score,
                status,
                parent_event_id,
                created_at,
                COUNT(*) OVER() AS total_count
            FROM news_event
            WHERE 1 = 1
        "#,
    );

    if let Some(filter) = &query.filter {
        if let Some(event_date) = &filter.event_date {
            qb.push(" AND event_date = ");
            qb.push_bind(event_date.as_str());
        }
        if let Some(status) = filter.status {
            qb.push(" AND status = ");
            qb.push_bind(status);
        }
    }

    qb.push(" ORDER BY event_date DESC, score DESC ");

    let page_size = query.page_size.unwrap_or(20);
    let page = query.page.unwrap_or(1).max(1);

    qb.push(" LIMIT ");
    qb.push_bind(page_size as i64);
    qb.push(" OFFSET ");
    qb.push_bind(((page - 1) * page_size) as i64);

    result.page = page;
    result.page_size = page_size;

    let rows: Vec<Row> = qb.build_query_as().fetch_all(db_pool).await.map_err(|e| {
        tracing::error!("查询新闻热点事件失败: {e:?}");
        ApiError::Database("数据库查询失败".into())
    })?;

    let total = rows.first().map(|r| r.total_count).unwrap_or(0);
    result.total_count = total as usize;
    result.total_pages = if page_size > 0 {
        ((total as f64) / (page_size as f64)).ceil() as u32
    } else {
        0
    };
    result.items = rows
        .into_iter()
        .map(|r| NewsEventDTO {
            id: r.id,
            event_date: r.event_date,
            cluster_id: r.cluster_id,
            title: r.title,
            summary: r.summary,
            news_count: r.news_count,
            score: r.score,
            status: r.status,
            parent_event_id: r.parent_event_id,
            created_at: r.created_at,
        })
        .collect();

    Ok(result)
}

/// 查询某事件下的新闻条目列表
pub async fn list_news_items_by_event(
    event_id: i64,
    db_pool: &PgPool,
) -> Result<Vec<NewsItemResponseDTO>> {
    let rows = sqlx::query_as::<_, NewsItemResponseDTO>(
        r#"
            SELECT
                ni.id,
                ni.item_id,
                ni.title,
                ni.url,
                ni.source,
                ni.published_at,
                ni.cluster_id,
                ni.extracted,
                ni.created_at
            FROM news_item ni
            INNER JOIN news_event_item nei ON nei.news_id = ni.id
            WHERE nei.event_id = $1
            ORDER BY ni.published_at DESC
        "#,
    )
    .bind(event_id)
    .fetch_all(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("查询事件 {event_id} 下的新闻条目失败: {e:?}");
        ApiError::Database("数据库查询失败".into())
    })?;

    Ok(rows)
}
