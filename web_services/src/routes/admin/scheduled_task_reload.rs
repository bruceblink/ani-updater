use crate::common::AppState;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, put, web};
use common::api::{ApiError, ApiResponse};
use common::po::ApiResult;
use common::utils::JwtClaims;

// 在路由中添加
#[put("/task/reload")]
async fn task_reload(req: HttpRequest, app_state: web::Data<AppState>) -> ApiResult {
    let claims = req
        .extensions()
        .get::<JwtClaims>()
        .cloned()
        .ok_or_else(|| ApiError::Unauthorized("未授权".into()))?;

    if !claims.roles.contains(&"admin".to_string()) {
        return Err(ApiError::Forbidden("需要管理员权限".into()));
    }
    match app_state.task_manager.refresh_config().await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::ok(serde_json::json!(
            {"status": "success",
            "message": "定时任务配置配置已刷新"}
        )))),
        Err(e) => {
            tracing::error!("数据库查询错误: {e:?}");
            Err(ApiError::Internal("定时任务配置刷新失败".into()))
        }
    }
}
