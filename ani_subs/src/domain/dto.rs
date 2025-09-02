use chrono::Utc;
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

#[derive(Debug, Clone)]
pub struct NewUser {
    pub email: String,
    pub username: String,
    pub password: String,
    pub display_name: String,
    pub avatar_url: String,
}

/// 用户身份Dto,用于关联第三方登录认证的数据
#[derive(Serialize, Debug, Clone)]
pub struct UserIdentityDto {
    pub provider_user_id: String,
    pub provider: String,
    pub email: Option<String>,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
}
