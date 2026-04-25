use crate::common::AppState;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, put, web};
use common::api::{ApiError, ApiResponse};
use common::po::ApiResult;
use common::utils::JwtClaims;

#[put("/task/reload")]
async fn task_reload(req: HttpRequest, app_state: web::Data<AppState>) -> ApiResult {
    let claims = req
        .extensions()
        .get::<JwtClaims>()
        .cloned()
        .ok_or_else(|| ApiError::Unauthorized("未授权".into()))?;

    let permissions: Vec<String> = sqlx::query_scalar(
        r#"
            SELECT DISTINCT p.name
            FROM permissions p
            JOIN role_permissions rp ON rp.permission_id = p.id
            JOIN user_roles ur ON ur.role_id = rp.role_id
            WHERE ur.user_id = $1
        "#,
    )
    .bind(claims.uid)
    .fetch_all(&app_state.db_pool)
    .await
    .map_err(|e| {
        tracing::error!("查询用户权限失败: {e}");
        ApiError::Internal("服务器内部错误".into())
    })?;

    if !claims.roles.iter().any(|r| r == "admin") && !permissions.iter().any(|p| p == "admin:all") {
        return Err(ApiError::Forbidden("需要管理员权限".into()));
    }

    match app_state.task_manager.refresh_config().await {
        Ok(()) => Ok(HttpResponse::Ok().json(ApiResponse::<()>::ok(()))),
        Err(e) => {
            tracing::error!("数据库查询错误: {e:?}");
            Err(ApiError::Internal("定时任务配置刷新失败".into()))
        }
    }
}
