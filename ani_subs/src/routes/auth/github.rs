use crate::common::{ACCESS_TOKEN, GITHUB_USER_AGENT, REFRESH_TOKEN};
use crate::configuration::Setting;
use crate::service::github_user_register;
use actix_web::{HttpResponse, Responder, cookie::Cookie, get, web};
use common::api::{ApiError, ApiResult};
use common::utils::{GithubUser, generate_jwt, generate_refresh_token};
use lazy_static::lazy_static;
use oauth2::{
    AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, Scope, TokenResponse,
    basic::BasicClient,
};
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::{collections::HashMap, env};

static HTTP: Lazy<Client> = Lazy::new(Client::new);

lazy_static! {
    static ref SECRET: String = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
}

lazy_static! {
    static ref ALLOWED_REDIRECT_URIS: Vec<&'static str> = vec![
        "http://localhost:5173",
        "http://localhost:3039",
        "https://app.example.com",
    ];
}

#[derive(Serialize, Deserialize)]
struct StateClaims {
    redirect_uri: String,
    pkce_verifier: String,
    exp: usize, // UNIX timestamp
}

/**
    GitHub 第三方登录的 API <br>
    /auth/github/login Get 请求 <br>
    url请求参数:  redirect_uri=xxxx
*/
#[get("/auth/github/login")]
async fn auth_github_login(
    data: web::Data<BasicClient>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    // 获取前端重定向的URL
    let redirect_uri = match query.get("redirect_uri")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        //.filter(|s| ALLOWED_REDIRECT_URIS.contains(s)) // 可选白名单
    {
        Some(uri) => uri,
        None => return HttpResponse::BadRequest().body("Invalid redirect_uri"),
    };

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // 生成 JWT state
    let exp = chrono::Utc::now().timestamp() as usize + 300; // 5分钟过期
    let claims = StateClaims {
        redirect_uri: redirect_uri.to_string(),
        pkce_verifier: pkce_verifier.secret().to_string(),
        exp,
    };

    let state_jwt = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(SECRET.as_ref()),
    )
    .unwrap();

    let (auth_url, _csrf_token) = data
        .get_ref()
        .authorize_url(move || CsrfToken::new(state_jwt))
        .add_scope(Scope::new("read:user".into()))
        .add_scope(Scope::new("user:email".into()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    HttpResponse::Found()
        .append_header(("Location", auth_url.to_string()))
        .finish()
}

/**
    GitHub 第三方登录的回调 API <br>
    /auth/github/callback Get 请求 <br>
    url请求参数:  code=xxxx&state=xxxx
*/
#[get("/auth/github/callback")]
async fn auth_github_callback(
    data: web::Data<BasicClient>,
    query: web::Query<HashMap<String, String>>,
    pool: web::Data<PgPool>,
    config: web::Data<Setting>,
) -> ApiResult {
    let code = query.get("code").cloned().unwrap_or_default();
    let state_jwt = query.get("state").cloned().unwrap_or_default();

    // 解码 state JWT
    let token_data = jsonwebtoken::decode::<StateClaims>(
        &state_jwt,
        &jsonwebtoken::DecodingKey::from_secret(SECRET.as_ref()),
        &jsonwebtoken::Validation::default(),
    )
    .map_err(|_| ApiError::Internal("Invalid state".into()))?;

    let redirect_uri = token_data.claims.redirect_uri;
    let pkce_verifier = PkceCodeVerifier::new(token_data.claims.pkce_verifier);

    let github_token_resp = data
        .get_ref()
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|_| ApiError::Internal("换取 github 的 access_token 失败".into()))?;

    let mut user: GithubUser = HTTP
        .get("https://api.github.com/user")
        .bearer_auth(github_token_resp.access_token().secret())
        .header("User-Agent", GITHUB_USER_AGENT)
        .send()
        .await
        .map_err(|_| ApiError::OAuth("获取GitHub用户信息失败".into()))?
        .json()
        .await
        .map_err(|_| ApiError::Internal("GitHub 用户信息解析失败".into()))?;

    if user.email.is_none()
        && let Ok(resp) = HTTP
            .get("https://api.github.com/user/emails")
            .bearer_auth(github_token_resp.access_token().secret())
            .header("User-Agent", GITHUB_USER_AGENT)
            .send()
            .await
        && let Ok(emails) = resp.json::<Vec<serde_json::Value>>().await
        && let Some(email) = emails.first()
    {
        user.email = email
            .get("email")
            .and_then(|e| e.as_str())
            .map(String::from);
    }

    let jwt_access = generate_jwt(&user, config.token[ACCESS_TOKEN] as i64)
        .map_err(|_| ApiError::Internal("access_token 生成失败".into()))?;

    let jwt_refresh = generate_refresh_token(config.token[REFRESH_TOKEN] as i64)
        .map_err(|_| ApiError::Internal("refresh_token 生成失败".into()))?;

    let refresh_token_string = github_user_register(
        pool,
        user.clone(),
        Some(github_token_resp.access_token().secret().to_string()),
        jwt_refresh,
    )
    .await
    .map_err(|_| ApiError::Internal("refresh_token 持久化失败".into()))?;

    // 生成 access_token的cookie
    let access_cookie = Cookie::build(ACCESS_TOKEN, jwt_access.clone().token)
        .http_only(true)
        .secure(true) // 生产环境必须 https
        .path("/")
        .same_site(actix_web::cookie::SameSite::None) // 为None时可以跨站点请求携带 Cookie
        .finish();
    // 生成 refresh_token的cookie
    let refresh_cookie = Cookie::build(REFRESH_TOKEN, refresh_token_string)
        .http_only(true)
        .secure(true) // 生产环境必须 https
        .path("/")
        .same_site(actix_web::cookie::SameSite::None) // 为None时可以跨站点请求携带 Cookie
        .finish();
    // 为了保险，防止浏览器(例如firefox的权限就比较严格，不一定会携带access_cookie)不携带access_cookie，
    // 最终重定向到前端传来的 redirect_uri
    let final_redirect_url = format!("{redirect_uri}?token={}", jwt_access.token);

    Ok(HttpResponse::Found()
        .append_header(("Location", final_redirect_url))
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .finish())
}
