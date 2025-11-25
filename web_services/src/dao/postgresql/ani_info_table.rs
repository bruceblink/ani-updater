use crate::domain::dto::AniInfoDto;
use crate::domain::po::{AniInfo, QueryPage};
use crate::routes::AniFilter;
use actix_web::web;
use anyhow::Result;
use chrono::Utc;
use chrono_tz::Asia::Shanghai;
use common::api::{AniItem, ApiError, PageData};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

/// 动漫信息插入新记录
pub async fn upsert_ani_info(item: &AniItem, db_pool: &PgPool) -> Result<()> {
    let _ = sqlx::query(
        r#"
        INSERT INTO ani_info (
            title,
            update_count,
            update_info,
            image_url,
            detail_url,
            platform
        ) VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (title, platform, update_count) DO UPDATE SET
            update_info = EXCLUDED.update_info,
            image_url = EXCLUDED.image_url,
            detail_url = EXCLUDED.detail_url
        "#,
    )
    .bind(&item.title)
    .bind(&item.update_count)
    .bind(&item.update_info)
    .bind(&item.image_url)
    .bind(&item.detail_url)
    .bind(&item.platform)
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("插入或更新 ani_info {:?} 失败: {}", item, e);
        anyhow::anyhow!(e)
    })?;

    Ok(())
}

/// 根据 id 查询单条
pub async fn get_ani_info_by_id(id: i64, db_pool: &PgPool) -> Result<Option<AniInfoDto>> {
    let rec = sqlx::query_as::<_, AniInfo>(
        r#"
                SELECT id,
                    title,
                    update_count,
                    update_info,
                    image_url,
                    detail_url,
                    update_time,
                    platform
                FROM ani_info
                WHERE
                  id = $1
            ;"#,
    )
    .bind(id)
    .fetch_optional(db_pool)
    .await?;
    // 转换时间字段到上海时区
    let dto = rec.map(|ani| AniInfoDto {
        id: ani.id,
        title: ani.title,
        update_count: ani.update_count,
        update_info: ani.update_info,
        image_url: ani.image_url,
        detail_url: ani.detail_url,
        update_time: ani.update_time.with_timezone(&Shanghai).to_rfc3339(),
        platform: ani.platform,
    });
    Ok(dto)
}

// 定义包含 AniInfo 和 total_count 的结构体
#[derive(Debug, FromRow, Clone)]
struct AniInfoWithTotal {
    pub id: i64,
    pub title: String,
    pub update_count: String,
    pub update_info: String,
    pub image_url: String,
    pub detail_url: String,
    pub update_time: chrono::DateTime<Utc>,
    pub platform: String,
    pub total_count: i64,
}

/// 查询所有动漫信息
pub async fn list_all_ani_info(
    query: web::Query<QueryPage<AniFilter>>,
    db_pool: &PgPool,
) -> Result<PageData<AniInfoDto>> {
    // 构造带绑定参数的 QueryAs
    let mut result = PageData {
        items: vec![],
        total_count: 0,
        page: 0,
        page_size: 0,
        total_pages: 0,
    };

    let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        r#"SELECT id,
                  title,
                  update_count,
                  update_info,
                  image_url,
                  detail_url,
                  update_time,
                  platform,
                  COUNT(*) OVER() as total_count
           FROM ani_info WHERE update_time >= current_date"#,
    );

    if let Some(filter) = &query.filter {
        if let Some(title) = &filter.title {
            query_builder.push(" AND title LIKE ");
            query_builder.push_bind(format!("%{title}%"));
        }
        if let Some(platform) = &filter.platform {
            query_builder.push(" AND platform = ");
            query_builder.push_bind(platform);
        }
    }

    query_builder.push(" ORDER BY update_time DESC");

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
    let rows: Vec<AniInfoWithTotal> = query_builder
        .build_query_as()
        .fetch_all(db_pool)
        .await
        .map_err(|e| {
            tracing::error!("数据库查询错误: {e:?}");
            ApiError::Database("数据库查询失败".into())
        })?;
    // 转换数据库数据为前端需要的的DTO数据
    let data: Vec<AniInfoDto> = rows
        .iter()
        .map(|ani| AniInfoDto {
            id: ani.id,
            title: ani.title.clone(),
            update_count: ani.update_count.clone(),
            update_info: ani.update_info.clone(),
            image_url: ani.image_url.clone(),
            detail_url: ani.detail_url.clone(),
            update_time: ani.update_time.with_timezone(&Shanghai).to_rfc3339(),
            platform: ani.platform.clone(),
        })
        .collect::<Vec<AniInfoDto>>();

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
