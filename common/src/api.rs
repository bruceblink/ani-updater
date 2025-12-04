use actix_web::{HttpResponse, Responder, ResponseError};
use anyhow::Error as AnyError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
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

/// 定义 Api统一返回结构对象
pub type ApiResult = Result<HttpResponse, ApiError>;

/// 分页数据结构
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PageData<T> {
    pub items: Vec<T>,      // 当前页的数据
    pub total_count: usize, // 总条数
    pub page: u32,          // 当前页码（1开始）
    pub page_size: u32,     // 每页数量
    pub total_pages: u32,   // 总页数
}

pub type ItemResult = HashMap<String, Vec<TaskItem>>;

#[derive(Debug, Clone)]
pub enum TaskItem {
    Ani(AniItem),
    Video(VideoItem),
    News(NewsInfo),
    Health(HealthItem),
    Extract(NewsInfo2Item),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AniItem {
    pub title: String,
    pub update_count: String,
    pub update_info: String,
    pub image_url: String,
    pub detail_url: String,
    pub update_time: String,
    pub platform: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BaseVideo {
    pub id: String,                    // id
    pub title: String,                 // 标题
    pub rating: Option<Value>,         // 评分
    pub pic: Option<Value>,            // 图片
    pub is_new: Option<bool>,          // 是否新上映
    pub uri: String,                   // 豆瓣地址
    pub episodes_info: Option<String>, // 更新集数信息
    pub card_subtitle: String,         // 副标题
    pub r#type: String,                // 类型 tv/movie/等   type 是关键字，需加 r#
}

pub type VideoItem = BaseVideo;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsInfo {
    pub id: String,
    pub name: String,
    pub items: Vec<Value>, // 不关心内部结构，直接用 Value 保存
}

/// 健康检测返回的结果集
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthItem {
    pub url: String,
    pub result: Value, // 不关心内部结构，直接用 Value 保存
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
