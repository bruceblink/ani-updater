use crate::common::AppState;
use actix_web::{HttpRequest, HttpResponse, get, web};
use common::TaskFilter;
use common::api::{ApiError, ApiResponse};
use common::po::{ApiResult, QueryPage};
use infra::list_all_scheduled_tasks_by_page;

#[get("/scheduledTasks")]
async fn scheduled_tasks_get(
    _: HttpRequest,
    query: web::Query<QueryPage<TaskFilter>>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    match list_all_scheduled_tasks_by_page(query, &app_state.db_pool).await {
        Ok(news) => Ok(HttpResponse::Ok().json(ApiResponse::ok(news))),
        Err(e) => {
            tracing::error!("数据库查询错误: {e:?}");
            Err(ApiError::Database("数据库查询失败".into()))
        }
    }
}
