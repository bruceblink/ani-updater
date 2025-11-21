use crate::dao::list_all_scheduled_tasks_by_page;
use crate::domain::po::QueryPage;
use actix_web::{HttpRequest, HttpResponse, get, web};
use common::api::{ApiError, ApiResponse, ApiResult};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// 定义"News"的嵌套的查询参数结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskFilter {
    pub name: Option<String>,
    pub arg: Option<String>,
    pub cmd: Option<String>,
}

#[get("/scheduledTasks")]
async fn scheduled_tasks_get(
    _: HttpRequest,
    query: web::Query<QueryPage<TaskFilter>>,
    db_pool: web::Data<PgPool>,
) -> ApiResult {
    match list_all_scheduled_tasks_by_page(query, &db_pool).await {
        Ok(news) => Ok(HttpResponse::Ok().json(ApiResponse::ok(news))),
        Err(e) => {
            tracing::error!("数据库查询错误: {e:?}");
            Err(ApiError::Database("数据库查询失败".into()))
        }
    }
}
