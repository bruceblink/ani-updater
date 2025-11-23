use crate::common::AppState;
use actix_web::{HttpResponse, get, web};
use common::api::{ApiError, ApiResponse, ApiResult};

// 在路由中添加
#[get("/task/reload")]
async fn task_reload(app_state: web::Data<AppState>) -> ApiResult {
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
