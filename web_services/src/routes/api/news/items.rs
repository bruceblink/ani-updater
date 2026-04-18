use crate::common::AppState;
use actix_web::{HttpResponse, get, web};
use common::NewsItemFilter;
use common::api::{ApiError, ApiResponse};
use common::po::{ApiResult, QueryPage};
use infra::list_all_news_item_by_page;

/// GET /api/news/items — 分页查询新闻条目列表
#[get("/news/items")]
async fn news_items_get(
    query: web::Query<QueryPage<NewsItemFilter>>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    match list_all_news_item_by_page(query, &app_state.db_pool).await {
        Ok(page) => Ok(HttpResponse::Ok().json(ApiResponse::ok(page))),
        Err(e) => {
            tracing::error!("查询新闻条目失败: {e:?}");
            Err(ApiError::Database("数据库查询失败".into()))
        }
    }
}
