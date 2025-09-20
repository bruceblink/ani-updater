use crate::domain::dto::AniInfoDto;
use chrono::Utc;
use common::api::BaseVideo;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;

#[derive(Debug, Clone, FromRow, Deserialize, Serialize)]
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
