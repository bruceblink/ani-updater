use crate::domain::dto::AniInfoDto;
use chrono::Utc;
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
