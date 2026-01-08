use chrono::Utc;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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
    pub tenant_id: Option<chrono::DateTime<Utc>>,
    pub org_id: String,
    pub plan: String,
    pub token_version: i64,
    pub status: String,
    pub locked_until: Option<chrono::DateTime<Utc>>,
    pub failed_login_attempts: i64,
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
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsInfoDTO {
    pub id: i64,
    pub news_from: String,
    pub news_date: chrono::NaiveDate,
    pub data: serde_json::Value,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: Option<chrono::DateTime<Utc>>,
    pub name: String,
    pub extracted: bool,
    pub extracted_at: Option<chrono::DateTime<Utc>>,
}

pub struct NewsItemDTO {
    pub id: String,
    pub title: String,
    pub url: String,
    pub content: serde_json::Value,
    pub source: chrono::DateTime<Utc>,
    pub published_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTasksDTO {
    pub id: i64,
    pub name: String,
    pub cron: String,
    pub params: serde_json::Value,
    pub is_enabled: bool,
    pub retry_times: u8,
    pub last_run: Option<chrono::DateTime<Utc>>,
    pub next_run: Option<chrono::DateTime<Utc>>,
    pub last_status: String,
}
