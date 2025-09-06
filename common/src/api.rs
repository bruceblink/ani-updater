use actix_web::{HttpResponse, Responder, ResponseError};
use anyhow::Error as AnyError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Serialize, Debug, Clone)]
pub struct ApiResponse<T = serde_json::Value> {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            status: "ok".into(),
            message: None,
            data: Some(data),
        }
    }

    pub fn err<E: ToString>(msg: E) -> Self {
        Self {
            status: "error".into(),
            message: Some(msg.to_string()),
            data: None,
        }
    }
}

impl<T: Serialize> Responder for ApiResponse<T> {
    type Body = actix_web::body::BoxBody;

    fn respond_to(self, _: &actix_web::HttpRequest) -> HttpResponse<Self::Body> {
        HttpResponse::Ok().json(self)
    }
}

/// 定义错误类型
#[derive(Debug, Error)]
pub enum ApiError {
    // OAuth / 第三方接口错误
    #[error("OAuth error: {0}")]
    OAuth(String),

    // 数据库相关错误
    #[error("Database error: {0}")]
    Database(String),

    // 权限问题
    #[error("Unauthorized")]
    Unauthorized(String),

    // 未找到资源
    #[error("Not Found")]
    NotFound(String),

    // 未分类/内部错误
    #[error("Internal Server Error")]
    Internal(String),
}

// 自动把 `anyhow::Error` 转成 `ApiError`
impl From<AnyError> for ApiError {
    fn from(_err: AnyError) -> Self {
        ApiError::Internal("内部错误".into())
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::OAuth(msg) => HttpResponse::BadGateway()
                .json(ApiResponse::<()>::err(format!("OAuth 失败: {msg}"))),
            ApiError::Database(msg) => HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::err(format!("数据库错误: {msg}"))),
            ApiError::Unauthorized(msg) => {
                HttpResponse::Unauthorized().json(ApiResponse::<()>::err(format!("未授权: {msg}")))
            }
            ApiError::NotFound(msg) => {
                HttpResponse::NotFound().json(ApiResponse::<()>::err(format!("资源未找到: {msg}")))
            }
            ApiError::Internal(msg) => HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::err(format!("服务器内部错误: {msg}"))),
        }
    }
}

/// 定义 Api统一返回结构对象
pub type ApiResult = Result<HttpResponse, ApiError>;

/// 分页数据结构
#[derive(Serialize, Debug)]
pub struct PageData<T> {
    pub items: Vec<T>,  // 当前页的数据
    pub total: usize,   // 总条数
    pub page: i64,      // 当前页码（1开始）
    pub page_size: i64, // 每页数量
}

pub type AniItemResult = HashMap<String, Vec<AniItem>>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AniItem {
    pub title: String,
    pub update_count: String,
    pub update_info: String,
    pub image_url: String,
    pub detail_url: String,
    pub update_time: String,
    pub platform: String,
}
