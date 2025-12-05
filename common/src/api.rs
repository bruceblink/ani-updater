use actix_web::{HttpResponse, Responder, ResponseError};
use anyhow::Error as AnyError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Serialize, Debug, Clone)]
pub struct ApiResponse<T = Value> {
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

    // 客户端请求参数问题
    #[error("Bad Request")]
    BadRequest(String),

    // 未找到资源
    #[error("Not Found")]
    NotFound(String),

    // 参数校验问题
    #[error("Invalid params")]
    InvalidData(String),

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
            ApiError::BadRequest(msg) => {
                HttpResponse::BadRequest().json(ApiResponse::<()>::err(format!("请求错误: {msg}")))
            }
            ApiError::InvalidData(msg) => HttpResponse::BadRequest()
                .json(ApiResponse::<()>::err(format!("参数校验错误: {msg}"))),
            ApiError::Internal(msg) => HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::err(format!("服务器内部错误: {msg}"))),
        }
    }
}

// NewsItem的结构体
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsInfo2Item {
    pub id: String,
    pub news_from: String,
    pub name: String,
    pub news_date: chrono::NaiveDate,
    pub news_item_id: String,
    pub title: String,
    pub url: String,
    pub content: Option<String>,
    pub extra: Value, // 不关心内部结构，直接用 Value 保存
}
