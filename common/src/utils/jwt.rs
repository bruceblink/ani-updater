use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
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

/// 校验 JWT
pub fn verify_jwt(token: &str) -> Option<Claims> {
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET 未设置");
    let decoded = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
    .ok()?;
    Some(decoded.claims)
}

// 生成 Access Token
/// 生成 JWT
/// `exp_minutes` 过期时间，单位分钟
/// 例如：60 * 2 表示 2 小时
pub fn generate_jwt(user: &GithubUser, exp_minutes: i64) -> String {
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET 未设置");
    let exp = chrono::Utc::now() + chrono::Duration::minutes(exp_minutes);

    let claims = Claims {
        sub: user.login.clone(),
        uid: user.id,
        exp: exp.timestamp() as usize,
        name: user.name.clone(),
        email: user.email.clone(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
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
    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET 未设置");
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}
