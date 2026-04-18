use actix_web::web;
use anyhow::Result;
use common::AniCollectFilter;
use common::api::ApiError;
use common::dto::{AniCollectDTO, CreateAniCollectDTO, WatchedAniCollectDTO};
use common::po::{AniCollect, PageData, QueryPage};
use sqlx::{PgPool, QueryBuilder};

/// 分页查询某用户的番剧收藏列表
pub async fn list_ani_collect_by_page(
    user_id: &str,
    query: web::Query<QueryPage<AniCollectFilter>>,
    db_pool: &PgPool,
) -> Result<PageData<AniCollectDTO>> {
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
        pub ani_item_id: i64,
        pub ani_title: String,
        pub collect_time: chrono::DateTime<chrono::Utc>,
        pub is_watched: bool,
        pub total_count: i64,
    }

    let mut query_builder: QueryBuilder<sqlx::postgres::Postgres> = QueryBuilder::new(
        r#"
            SELECT
                id,
                ani_item_id,
                ani_title,
                collect_time,
                is_watched,
                COUNT(*) OVER() AS total_count
            FROM ani_collect
            WHERE user_id =
        "#,
    );
    query_builder.push_bind(user_id);

    if let Some(filter) = &query.filter {
        if let Some(ani_title) = &filter.ani_title {
            query_builder.push(" AND ani_title LIKE ");
            query_builder.push_bind(format!("%{ani_title}%"));
        }
        if let Some(is_watched) = filter.is_watched {
            query_builder.push(" AND is_watched = ");
            query_builder.push_bind(is_watched);
        }
    }

    query_builder.push(" ORDER BY collect_time DESC ");

    let page_size = query.page_size.unwrap_or(20);
    let page = query.page.unwrap_or(1).max(1);

    query_builder.push(" LIMIT ");
    query_builder.push_bind(page_size as i64);
    query_builder.push(" OFFSET ");
    query_builder.push_bind(((page - 1) * page_size) as i64);

    result.page = page;
    result.page_size = page_size;

    let rows: Vec<Row> = query_builder
        .build_query_as()
        .fetch_all(db_pool)
        .await
        .map_err(|e| {
            tracing::error!("查询番剧收藏列表失败: {e:?}");
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
        .map(|r| AniCollectDTO {
            id: r.id,
            ani_item_id: r.ani_item_id,
            ani_title: r.ani_title,
            collect_time: r.collect_time,
            is_watched: r.is_watched,
        })
        .collect();

    Ok(result)
}

/// 添加番剧收藏
pub async fn create_ani_collect(
    user_id: &str,
    dto: &CreateAniCollectDTO,
    db_pool: &PgPool,
) -> Result<AniCollectDTO> {
    let row = sqlx::query_as::<_, AniCollect>(
        r#"
            INSERT INTO ani_collect (user_id, ani_item_id, ani_title)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, ani_item_id, ani_title, collect_time, is_watched
        "#,
    )
    .bind(user_id)
    .bind(dto.ani_item_id)
    .bind(&dto.ani_title)
    .fetch_one(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("添加番剧收藏失败: {e:?}");
        anyhow::anyhow!("添加收藏失败，可能已收藏该番剧")
    })?;

    Ok(AniCollectDTO {
        id: row.id,
        ani_item_id: row.ani_item_id,
        ani_title: row.ani_title,
        collect_time: row.collect_time,
        is_watched: row.is_watched,
    })
}

/// 取消番剧收藏
pub async fn delete_ani_collect(id: i64, user_id: &str, db_pool: &PgPool) -> Result<()> {
    let rows_affected = sqlx::query(r#"DELETE FROM ani_collect WHERE id = $1 AND user_id = $2"#)
        .bind(id)
        .bind(user_id)
        .execute(db_pool)
        .await
        .map_err(|e| {
            tracing::error!("取消番剧收藏 {id} 失败: {e:?}");
            anyhow::anyhow!("取消收藏失败")
        })?
        .rows_affected();

    if rows_affected == 0 {
        return Err(anyhow::anyhow!("收藏记录不存在或无权操作"));
    }
    Ok(())
}

/// 更新番剧观看状态
pub async fn update_ani_collect_watched(
    id: i64,
    user_id: &str,
    dto: &WatchedAniCollectDTO,
    db_pool: &PgPool,
) -> Result<()> {
    let rows_affected =
        sqlx::query(r#"UPDATE ani_collect SET is_watched = $1 WHERE id = $2 AND user_id = $3"#)
            .bind(dto.is_watched)
            .bind(id)
            .bind(user_id)
            .execute(db_pool)
            .await
            .map_err(|e| {
                tracing::error!("更新番剧观看状态 {id} 失败: {e:?}");
                anyhow::anyhow!("更新观看状态失败")
            })?
            .rows_affected();

    if rows_affected == 0 {
        return Err(anyhow::anyhow!("收藏记录不存在或无权操作"));
    }
    Ok(())
}
