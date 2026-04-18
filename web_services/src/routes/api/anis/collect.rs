use crate::common::AppState;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, delete, get, patch, post, web};
use common::AniCollectFilter;
use common::api::{ApiError, ApiResponse};
use common::dto::{CreateAniCollectDTO, WatchedAniCollectDTO};
use common::po::{ApiResult, QueryPage};
use common::utils::JwtClaims;
use infra::{
    create_ani_collect, delete_ani_collect, list_ani_collect_by_page, update_ani_collect_watched,
};

fn extract_user_id(req: &HttpRequest) -> Result<String, ApiError> {
    req.extensions()
        .get::<JwtClaims>()
        .map(|c| c.sub.clone())
        .ok_or_else(|| ApiError::Unauthorized("未授权".into()))
}

/// GET /api/anis/collect — 分页查询当前用户收藏列表
#[get("/anis/collect")]
async fn ani_collect_list(
    req: HttpRequest,
    query: web::Query<QueryPage<AniCollectFilter>>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    let user_id = extract_user_id(&req)?;
    match list_ani_collect_by_page(&user_id, query, &app_state.db_pool).await {
        Ok(page) => Ok(HttpResponse::Ok().json(ApiResponse::ok(page))),
        Err(e) => {
            tracing::error!("查询收藏列表失败: {e:?}");
            Err(ApiError::Database("数据库查询失败".into()))
        }
    }
}

/// POST /api/anis/collect — 添加收藏
#[post("/anis/collect")]
async fn ani_collect_create(
    req: HttpRequest,
    body: web::Json<CreateAniCollectDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    let user_id = extract_user_id(&req)?;
    match create_ani_collect(&user_id, &body, &app_state.db_pool).await {
        Ok(dto) => Ok(HttpResponse::Created().json(ApiResponse::ok(dto))),
        Err(e) => {
            tracing::error!("添加收藏失败: {e:?}");
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

/// DELETE /api/anis/collect/{id} — 取消收藏
#[delete("/anis/collect/{id}")]
async fn ani_collect_delete(
    req: HttpRequest,
    path: web::Path<i64>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    let user_id = extract_user_id(&req)?;
    let id = path.into_inner();
    match delete_ani_collect(id, &user_id, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => Err(ApiError::NotFound(e.to_string())),
    }
}

/// PATCH /api/anis/collect/{id}/watched — 标记观看状态
#[patch("/anis/collect/{id}/watched")]
async fn ani_collect_watched(
    req: HttpRequest,
    path: web::Path<i64>,
    body: web::Json<WatchedAniCollectDTO>,
    app_state: web::Data<AppState>,
) -> ApiResult {
    let user_id = extract_user_id(&req)?;
    let id = path.into_inner();
    match update_ani_collect_watched(id, &user_id, &body, &app_state.db_pool).await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => Err(ApiError::NotFound(e.to_string())),
    }
}
