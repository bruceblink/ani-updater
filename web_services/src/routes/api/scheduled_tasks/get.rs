use crate::common::AppState;
use crate::dao::list_all_scheduled_tasks_by_page;
use actix_web::{HttpRequest, HttpResponse, get, web};
use common::api::{ApiError, ApiResponse};
use common::po::{ApiResult, QueryPage};
use serde::{Deserialize, Serialize};

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
