use crate::common::ExtractToken;
use actix_web::{HttpRequest, HttpResponse, get, web};
use chrono::Utc;
use common::api::{ApiError, ApiResult};
use common::utils::verify_jwt;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct NewsInfoDTO {
    pub news_from: String,
    pub news_date: chrono::NaiveDate,
    pub data: serde_json::Value,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct NewsInfo {
    pub id: i64,
    pub news_from: String,
    pub news_date: chrono::NaiveDate,
    pub data: serde_json::Value,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchParams {
    news_from: String,
    news_date: String,
}

#[get("/news")]
async fn news_get(
    req: HttpRequest,
    query: web::Query<SearchParams>,
    db: web::Data<PgPool>,
) -> ApiResult {
    // 获取请求参数
    let news_from = &query.news_from.trim();
    let news_date = &query.news_date.trim();

    if let Some(token) = req.get_access_token()
        && let Ok(_) = verify_jwt(&token)
    {
        let rec = sqlx::query_as::<_, NewsInfo>(
            r#"
            SELECT ni.id, ni.news_from, ni.news_date, ni.data, ni.created_at, ni.updated_at
            FROM news_info ni
            WHERE ni.news_from = $1 AND ni.news_date = $2;
        "#,
        )
        .bind(news_from)
        .bind(news_date)
        .fetch_optional(db.get_ref())
        .await
        .map_err(|e| {
            tracing::error!("查询新闻信息报错: {e}");
            ApiError::Internal("服务器内部错误".into())
        })?;
        // 转换时间字段到上海时区
        let news_info_dto = rec.map(|news_info| NewsInfoDTO {
            news_from: news_info.news_from,
            news_date: news_info.news_date,
            data: news_info.data,
            created_at: news_info.created_at,
            updated_at: news_info.updated_at,
        });
        return Ok(HttpResponse::Ok().json(news_info_dto));
    }
    Err(ApiError::Unauthorized("请求参数不正确".into()))
}
