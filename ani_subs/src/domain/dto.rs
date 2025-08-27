use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AniInfoDto {
    pub id: i64,
    pub title: String,
    pub update_count: String,
    pub update_info: String,
    pub image_url: String,
    pub detail_url: String,
    pub update_time: String,
    pub platform: String,
}
