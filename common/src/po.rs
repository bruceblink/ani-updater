use crate::api::{ApiError, NewsInfo2Item};
use crate::dto::AniInfoDto;
use actix_web::HttpResponse;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use std::collections::HashMap;

#[derive(Debug, Clone, FromRow, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AniInfo {
    pub id: i64,
    pub title: String,
    pub update_count: String,
    pub update_info: String,
    pub image_url: String,
    pub detail_url: String,
    pub update_time: chrono::DateTime<Utc>,
    pub platform: String,
}

pub type AniResult = HashMap<String, Vec<AniInfoDto>>;

#[derive(Debug, Clone, FromRow, PartialEq, Deserialize, Serialize)]
pub struct AniCollect {
    pub id: i64,
    pub user_id: String,
    pub ani_item_id: i64,
    pub ani_title: String,
    pub collect_time: chrono::DateTime<Utc>,
    pub is_watched: bool,
}

#[derive(Debug, Clone, FromRow, PartialEq, Deserialize, Serialize)]
pub struct AniWatchHistory {
    pub id: i64,
    pub user_id: String,
    pub ani_item_id: i64,
    pub watched_time: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, PartialEq, Deserialize, Serialize)]
pub struct AniColl {
    pub user_id: String,
    pub ani_item_id: i64,
    pub ani_title: String,
    pub collect_time: chrono::DateTime<Utc>,
    pub is_watched: bool,
}

#[derive(Debug, Clone, FromRow, PartialEq, Deserialize, Serialize)]
pub struct AniWatch {
    pub user_id: String,
    pub ani_item_id: i64,
    pub watched_time: chrono::DateTime<Utc>,
}

#[derive(Serialize, Debug, Clone, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct AniHistoryInfo {
    id: i64,
    title: String,
    update_count: String,
    update_info: String,
    image_url: String,
    detail_url: String,
    is_watched: bool,
    user_id: String,
    update_time: chrono::DateTime<Utc>,
    watched_time: Option<chrono::DateTime<Utc>>,
    platform: String,
    pub total_count: i64,
}

#[derive(Serialize, Debug, Clone, FromRow)]
pub struct UserInfo {
    pub id: i64,
    pub email: String,
    pub username: String,
    pub password: String,
    pub display_name: String,
    pub avatar_url: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct VideoInfo {
    #[serde(flatten)]
    pub base: BaseVideo, // 继承 BaseVideo
    pub original_title: String,
    pub intro: String,
    pub director: serde_json::Value,
    pub screenwriter: Option<serde_json::Value>,
    pub actors: Option<serde_json::Value>,
    pub category: Option<serde_json::Value>, // 分类
    pub genres: Option<serde_json::Value>,
    pub production_country: Option<serde_json::Value>,
    pub language: Option<String>,
    pub release_year: Option<i32>,
    pub release_date: Option<serde_json::Value>,
    pub duration: serde_json::Value,
    pub aka: Option<serde_json::Value>,
    pub imdb: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rating {
    pub value: f64,               // 分数值
    pub count: Option<u32>,       // 评分人数
    pub max: Option<u32>,         // 最高分
    pub start_count: Option<u32>, // 起始评分人数
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pic {
    pub normal: String,        // 正常尺寸
    pub large: Option<String>, // 大图
}

/// 分页查询的查询条件
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryPage<T> {
    pub filter: Option<T>,
    //sort: Option<String>, // 例如 "price", "-price", "name,-price"
    pub page: Option<u32>,
    pub page_size: Option<u32>,
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
    ExtractNewsItem(NewsInfo2Item),
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

#[derive(Debug, FromRow, Clone)]
pub struct ScheduledTasks {
    #[allow(dead_code)]
    pub id: i64,
    pub name: String,
    pub cron: String,
    pub params: serde_json::Value,
    pub is_enabled: bool,
    pub retry_times: i16,
    pub last_run: Option<chrono::DateTime<Utc>>,
    pub next_run: Option<chrono::DateTime<Utc>>,
    pub last_status: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: Option<chrono::DateTime<Utc>>,
}
