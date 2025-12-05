use crate::common::AppState;
use crate::dao::list_all_news_info_by_page;
use actix_web::{HttpRequest, HttpResponse, get, web};
use chrono::Utc;
use common::api::{ApiError, ApiResponse};
use common::po::{ApiResult, QueryPage};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsInfoDTO {
    pub id: i64,
    pub news_from: String,
    pub news_date: chrono::NaiveDate,
    pub data: serde_json::Value,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: Option<chrono::DateTime<Utc>>,
    pub name: String,
    pub extracted: bool,
    pub extracted_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct NewsInfo {
    pub id: i64,
    pub news_from: String,
    pub name: String,
    pub news_date: chrono::NaiveDate,
    pub data: serde_json::Value,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: Option<chrono::DateTime<Utc>>,
    pub extracted: bool,
    pub extracted_at: Option<chrono::DateTime<Utc>>,
}

/// 定义"News"的嵌套的查询参数结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsFilter {
    pub news_from: Option<String>,
    pub news_date: Option<String>,
    pub extracted: Option<bool>,
}

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
