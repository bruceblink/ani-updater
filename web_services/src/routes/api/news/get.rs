use crate::common::AppState;
use actix_web::{HttpRequest, HttpResponse, get, web};
use common::NewsFilter;
use common::api::{ApiError, ApiResponse};
use common::po::{ApiResult, QueryPage};
use infra::list_all_news_info_by_page;

#[get("/news")]
async fn news_get(
    _: HttpRequest,
    query: web::Query<QueryPage<NewsFilter>>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    match list_all_news_info_by_page(query, &app_state.db_pool).await {
        Ok(new) => Ok(HttpResponse::Ok().json(ApiResponse::ok(new))),
        Err(e) => {
            tracing::error!("数据库查询错误: {e:?}");
            Err(ApiError::Database("数据库查询失败".into()))
        }
    }
}
