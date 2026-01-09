use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use once_cell::sync::Lazy;
use rand::Rng;
use rand::distr::Alphanumeric;
use serde::{Deserialize, Serialize};
use std::env;

/* ================= JWT SECRET ================= */

static JWT_SECRET: Lazy<Result<String>> =
    Lazy::new(|| env::var("JWT_SECRET").context("环境变量 JWT_SECRET 未设置"));

/* ================= 用户模型 ================= */

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GithubUser {
    pub login: String,
    pub id: i64,
    pub avatar_url: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommonUser {
    pub id: i64,                    // 系统用户id
    pub sub: String,                // 第三方登录用户名
    pub uid: i64,                   // 第三方登录 ID
    pub email: Option<String>,      // 邮箱
    pub avatar_url: Option<String>, // 图像
    pub r#type: String,             // 登录类型
    pub roles: Vec<String>,         // 角色
    pub ver: i64,                   // token 版本号
}

/* ================= JWT Claims ================= */

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    /* ========= JWT 标准字段（通用 / 安全 / 跨服务） ========= */
    /// subject：JWT 所代表的主体
    /// 通常使用 user_id 的字符串形式
    /// 👉 JWT 规范字段，很多中间件/网关默认依赖它
    pub sub: String,

    /// expiration time：过期时间（Unix 时间戳，秒）
    /// 👉 强制 token 生命周期，防止长期泄露风险
    pub exp: i64,

    /// issued at：签发时间（Unix 时间戳，秒）
    /// 👉 用于审计、风控、排查 token 异常
    pub iat: i64,

    /* ========= 业务字段（与你的系统强相关） ========= */
    /// 系统用户 ID（数值型）
    /// 👉 冗余字段，避免每次从 sub 再解析
    /// 👉 方便数据库查询与日志记录
    pub uid: i64,

    pub email: Option<String>,

    pub avatar: Option<String>,
    /// RBAC 角色列表
    /// 👉 角色相对稳定，适合放在 JWT 中
    /// 👉 用于服务端快速鉴权（是否允许访问接口）
    pub roles: Vec<String>,

    /// token 版本号（非常关键）
    /// 👉 用于主动失效 token
    /// 👉 密钥泄露 / 用户被禁用 / 角色变更时递增
    pub ver: i64,
}

/* ================= Token Struct ================= */

/// refresh_token的结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RefreshToken {
    pub token: String,
    pub expires_at: chrono::DateTime<Utc>,
}

/// access_token的结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccessToken {
    pub token: String,
    pub expires_at: chrono::DateTime<Utc>,
}

/// 获取 JWT_SECRET
fn jwt_secret() -> Result<&'static String> {
    JWT_SECRET
        .as_ref()
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}

/// 生成 Access Token
/// `exp_minutes` 过期时间，单位分钟
/// 例如：60 * 2 表示 2 小时
pub fn generate_jwt(user: &CommonUser, exp_minutes: i64) -> Result<AccessToken> {
    let secret = jwt_secret()?;
    let now = Utc::now();
    let exp = now + Duration::minutes(exp_minutes);

    let claims = JwtClaims {
        sub: user.sub.clone(),
        uid: user.id,
        email: user.email.clone(),
        avatar: user.avatar_url.clone(),
        roles: user.roles.clone(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
        ver: user.ver,
    };

    let token = encode(
        &Header::default(), // HS256
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .context("JWT 生成失败")?;

    Ok(AccessToken {
        token,
        expires_at: exp,
    })
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

/// 校验 + 解析 JWT（统一用这个）
pub fn verify_jwt(token: &str) -> Result<JwtClaims> {
    let secret = jwt_secret()?;

    let validation = Validation::default(); // 校验 exp / alg

    let data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .context("JWT 验证失败")?;

    Ok(data.claims)
}
