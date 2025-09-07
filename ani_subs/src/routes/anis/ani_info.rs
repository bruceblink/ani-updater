use crate::dao::get_ani_info_by_id;
use crate::dao::list_all_ani_info;
use actix_web::{HttpResponse, web};
use common::api::{ApiError, ApiResponse, ApiResult};
use sqlx::PgPool;

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

pub async fn get_anis(pool: web::Data<PgPool>) -> ApiResult {
    match list_all_ani_info("ani_id".to_string(), &pool).await {
        Ok(ani_list) => Ok(HttpResponse::Ok().json(ApiResponse::ok(ani_list))),
        Err(e) => {
            tracing::error!("数据库查询错误: {e:?}");
            Err(ApiError::Database("数据库查询失败".into()))
        }
    }
}
