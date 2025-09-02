use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use rand::Rng;
use rand::distr::Alphanumeric;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // GitHub login
    pub uid: u64,    // GitHub ID
    pub exp: usize,  // 过期时间戳
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GithubUser {
    pub login: String,
    pub id: u64,
    pub avatar_url: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
}

/// refresh_token的结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefreshToken {
    pub token: String,
    pub expires_at: chrono::DateTime<Utc>,
}

/// 获取 JWT_SECRET
fn jwt_secret() -> Result<String> {
    env::var("JWT_SECRET").context("环境变量 JWT_SECRET 未设置")
}

/// 校验 JWT
pub fn verify_jwt(token: &str) -> Result<Claims> {
    let secret = jwt_secret()?;
    let decoded = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
    .context("JWT 验证失败")?;

    Ok(decoded.claims)
}

// 生成 Access Token
/// 生成 JWT
/// `exp_minutes` 过期时间，单位分钟
/// 例如：60 * 2 表示 2 小时
pub fn generate_jwt(user: &GithubUser, exp_minutes: i64) -> Result<String> {
    let secret = jwt_secret()?;
    let exp = Utc::now() + Duration::minutes(exp_minutes);

    let claims = Claims {
        sub: user.login.clone(),
        uid: user.id,
        exp: exp.timestamp() as usize,
        name: user.name.clone(),
        email: user.email.clone(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
    .context("JWT 生成失败")?;

    Ok(token)
}

/// 生成 Refresh Token（JWT 也可以用 JWT，但加随机字符串更安全）
/// `exp_days`: token的有效期，单位天数
pub fn generate_refresh_token(exp_days: i64) -> Result<RefreshToken> {
    // 随机字符串 (64 字符，字母数字)
    let token: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    // 设置过期时间 (比如 30 天)
    let expires_at = Utc::now() + Duration::days(exp_days);

    Ok(RefreshToken { token, expires_at })
}

// 解析 Access Token
pub fn decode_jwt(token: &str) -> Result<Claims> {
    let secret = jwt_secret()?;
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
    .context("JWT 解析失败")?;

    Ok(token_data.claims)
}
