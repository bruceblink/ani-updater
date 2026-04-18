use crate::common::AppState;
use actix_web::{HttpResponse, get, web};
use common::NewsEventFilter;
use common::api::{ApiError, ApiResponse};
use common::po::{ApiResult, QueryPage};
use infra::{list_all_news_event_by_page, list_news_items_by_event};

/// GET /api/news/events — 分页查询新闻热点事件列表
#[get("/news/events")]
async fn news_events_get(
    query: web::Query<QueryPage<NewsEventFilter>>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    match list_all_news_event_by_page(query, &app_state.db_pool).await {
        Ok(page) => Ok(HttpResponse::Ok().json(ApiResponse::ok(page))),
        Err(e) => {
            tracing::error!("查询新闻热点事件失败: {e:?}");
            Err(ApiError::Database("数据库查询失败".into()))
        }
    }
}

/// GET /api/news/events/{id}/items — 查询某事件下的新闻条目
#[get("/news/events/{id}/items")]
async fn news_event_items_get(path: web::Path<i64>, app_state: web::Data<AppState>) -> ApiResult {
    let event_id = path.into_inner();
    match list_news_items_by_event(event_id, &app_state.db_pool).await {
        Ok(items) => Ok(HttpResponse::Ok().json(ApiResponse::ok(items))),
        Err(e) => {
            tracing::error!("查询事件 {event_id} 下的新闻条目失败: {e:?}");
            Err(ApiError::Database("数据库查询失败".into()))
        }
    }
}
