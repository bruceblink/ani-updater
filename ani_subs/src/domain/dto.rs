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

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub id: i64,
    pub email: String,
    pub username: String,
    pub password: String,
    pub display_name: String,
    pub avatar_url: String,
    pub created_at: String,
    pub updated_at: Option<String>,
}
