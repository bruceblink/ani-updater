use crate::common::AppState;
use crate::dao::{get_ani_info_by_id, list_all_ani_info};
use crate::domain::po::QueryPage;
use actix_web::{HttpResponse, web};
use common::api::{ApiError, ApiResponse, ApiResult};
use serde::Deserialize;

// 定义嵌套的查询参数结构
#[derive(Debug, Deserialize, Clone)]
pub struct AniFilter {
    pub title: Option<String>,
    pub platform: Option<String>,
}

pub async fn get_ani(path: web::Path<(i64,)>, app_state: web::Data<AppState>) -> ApiResult {
    let ani_id = path.into_inner().0;
    match get_ani_info_by_id(ani_id, &app_state.db_pool).await {
        Ok(Some(ani)) => Ok(HttpResponse::Ok().json(ApiResponse::ok(ani))),
        Ok(None) => Err(ApiError::NotFound("番剧信息未找到".into())),
        Err(e) => {
            tracing::error!("数据库查询错误: {e:?}");
            Err(ApiError::Database("数据库查询失败".into()))
        }
    }
}

pub async fn get_anis(
    query: web::Query<QueryPage<AniFilter>>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    match list_all_ani_info(query, &app_state.db_pool).await {
        Ok(page) => Ok(HttpResponse::Ok().json(ApiResponse::ok(page))),
        Err(e) => {
            tracing::error!("数据库查询错误: {e:?}");
            Err(ApiError::Database("数据库查询失败".into()))
        }
    }
}
