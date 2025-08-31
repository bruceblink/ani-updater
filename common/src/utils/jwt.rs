use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64, // 系统用户 ID
    pub github_id: Option<i64>,
    pub exp: usize, // Access token 过期时间
}

const JWT_SECRET: &[u8] = b"your-very-secret-key";
const JWT_EXP_HOURS: i64 = 24; // Access token 有效期
// const REFRESH_EXP_DAYS: i64 = 30;     // Refresh token 有效期

// 生成 Access Token
pub fn generate_jwt(user_id: i64, github_id: Option<i64>) -> String {
    let exp = (Utc::now() + Duration::hours(JWT_EXP_HOURS)).timestamp() as usize;
    let claims = Claims {
        sub: user_id,
        github_id,
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
    .unwrap()
}

// 生成 Refresh Token（JWT 也可以用 JWT，但加随机字符串更安全）
pub fn generate_refresh_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let token: String = (0..64)
        .map(|_| rng.sample(rand::distributions::Alphanumeric) as char)
        .collect();
    token
}

// 解析 Access Token
pub fn decode_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}
