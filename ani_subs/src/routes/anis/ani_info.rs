use crate::dao::get_ani_info_by_id;
use crate::domain::po::AniInfo;
use actix_web::{HttpResponse, web};
use common::api::{ApiError, ApiResponse, ApiResult, PageData};
use serde::Deserialize;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder, Row};

// 定义嵌套的查询参数结构
#[derive(Debug, Deserialize, Clone)]
pub struct Filter {
    title: Option<String>,
    platform: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AniQuery {
    filter: Option<Filter>,
    //sort: Option<String>, // 例如 "price", "-price", "name,-price"
    page: Option<u32>,
    page_size: Option<u32>,
}

pub async fn get_ani(path: web::Path<(i64,)>, pool: web::Data<PgPool>) -> ApiResult {
    let ani_id = path.into_inner().0;
    match get_ani_info_by_id(ani_id, &pool).await {
        Ok(Some(ani)) => Ok(HttpResponse::Ok().json(ApiResponse::ok(ani))),
        Ok(None) => Err(ApiError::NotFound("番剧信息未找到".into())),
        Err(e) => {
            tracing::error!("数据库查询错误: {e:?}");
            Err(ApiError::Database("数据库查询失败".into()))
        }
    }
}

pub async fn get_anis(query: web::Query<AniQuery>, pool: web::Data<PgPool>) -> ApiResult {
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

    let rows = query_builder
        .build()
        .fetch_all(pool.get_ref())
        .await
        .map_err(|e| {
            tracing::error!("数据库查询错误: {e:?}");
            ApiError::Database("数据库查询失败".into())
        })?;

    let data: Vec<AniInfo> = rows
        .iter()
        .map(AniInfo::from_row)
        .collect::<Result<_, _>>()
        .map_err(|e| {
            tracing::error!("数据转换错误: {e:?}");
            ApiError::Database("数据转换失败".into())
        })?;
    result.items = data;
    let total_count = if rows.is_empty() {
        0
    } else {
        rows[0].get::<i64, _>("total_count")
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
    // 返回 JSON 响应
    Ok(HttpResponse::Ok().json(ApiResponse::ok(result)))
}
