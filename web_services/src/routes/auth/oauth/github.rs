use crate::common::{AppState, GITHUB_USER_AGENT};
use actix_web::{HttpResponse, cookie::Cookie, get, web};
use common::api::ApiError;
use common::po::ApiResult;
use common::utils::GithubUser;
use common::{ACCESS_TOKEN, REFRESH_TOKEN};
use lazy_static::lazy_static;
use oauth2::basic::BasicClient;
use oauth2::{
    AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, Scope, TokenResponse,
};
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use service::register_service::github_user_register;
use std::collections::HashMap;
use tracing::error;

static HTTP: Lazy<Client> = Lazy::new(Client::new);

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
#[get("/auth/oauth/github/login")]
async fn auth_github_login(
    app_state: web::Data<AppState>,
    query: web::Query<HashMap<String, String>>,
) -> ApiResult {
    // 1. 校验 redirect_uri
    let redirect_uri = query
        .get("redirect_uri")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        //.filter(|s| ALLOWED_REDIRECT_URIS.contains(s)) // 可选白名单
        .ok_or_else(|| ApiError::BadRequest("Invalid redirect_uri".into()))?;

    // 2. 生成 GitHub 授权地址
    let auth_url = get_github_authorization_url(
        &app_state.oauth_client,
        redirect_uri,
        &app_state.oauth_config.jwt_secret,
    )
    .await
    .map_err(|e| {
        error!("GitHub auth url generate failed: {:?}", e);
        ApiError::Internal("生成 GitHub 授权地址失败".into())
    })?;

    // 3. 302 跳转
    Ok(HttpResponse::Found()
        .append_header(("Location", auth_url))
        .finish())
}

/// 生成 GitHub 授权地址
pub async fn get_github_authorization_url(
    oauth_client: &BasicClient,
    redirect_uri: &str,
    jwt_secret: &String,
) -> anyhow::Result<String> {
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
        &jsonwebtoken::EncodingKey::from_secret(jwt_secret.as_ref()),
    )?;

    // 获取授权 URL
    let (auth_url, _) = oauth_client
        .authorize_url(move || CsrfToken::new(state_jwt))
        .add_scope(Scope::new("read:user".into()))
        .add_scope(Scope::new("user:email".into()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    Ok(auth_url.to_string())
}

/**
    GitHub 第三方登录的回调 API <br>
    /auth/oauth/github/callback Get 请求 <br>
    url请求参数:  code=xxxx&state=xxxx
*/
#[get("/auth/oauth/github/callback")]
async fn auth_github_callback(
    app_state: web::Data<AppState>,
    query: web::Query<HashMap<String, String>>,
) -> ApiResult {
    // 1. 取 code
    let code = query
        .get("code")
        .cloned()
        .ok_or_else(|| ApiError::BadRequest("missing code".into()))?;

    // 2. 取 state
    let state = query
        .get("state")
        .cloned()
        .ok_or_else(|| ApiError::BadRequest("missing state".into()))?;

    // 解码 state JWT
    let token_data = jsonwebtoken::decode::<StateClaims>(
        &state,
        &jsonwebtoken::DecodingKey::from_secret(app_state.oauth_config.jwt_secret.as_ref()),
        &jsonwebtoken::Validation::default(),
    )
    .map_err(|_| ApiError::Internal("Invalid state".into()))?;

    let pkce_verifier = PkceCodeVerifier::new(token_data.claims.pkce_verifier);

    // 获取GitHub的用户信息
    let user = get_github_user_info(&app_state.oauth_client, code.clone(), pkce_verifier).await?;

    // 注册“使用GitHub登录的用户”为系统用户
    let (access_token, refresh_token) = github_user_register(
        &app_state.db_pool.clone(),
        &app_state.configuration,
        user.clone(),
    )
    .await
    .map_err(|_| ApiError::Internal("github用户注册为系统用户失败".into()))?;

    // 生成 access_token的cookie
    let access_cookie = Cookie::build(ACCESS_TOKEN, access_token.clone().token)
        .http_only(true)
        .secure(true) // 生产环境必须 https
        .path("/")
        .same_site(actix_web::cookie::SameSite::None) // 为None时可以跨站点请求携带 Cookie
        .finish();
    // 生成 refresh_token的cookie
    let refresh_cookie = Cookie::build(REFRESH_TOKEN, refresh_token.token)
        .http_only(true)
        .secure(true) // 生产环境必须 https
        .path("/")
        .same_site(actix_web::cookie::SameSite::None) // 为None时可以跨站点请求携带 Cookie
        .finish();
    // 为了保险，防止浏览器(例如firefox的权限就比较严格，不一定会携带access_cookie)不携带access_cookie，
    // 最终重定向到前端传来的 redirect_uri
    let redirect_uri = token_data.claims.redirect_uri;
    let final_redirect_url = format!("{redirect_uri}?token={}", access_token.token);

    Ok(HttpResponse::Found()
        .append_header(("Location", final_redirect_url))
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .finish())
}

/// 获取GitHub用户信息
pub async fn get_github_user_info(
    oauth_client: &BasicClient,
    code: String,
    pkce_verifier: PkceCodeVerifier,
) -> anyhow::Result<GithubUser> {
    // 获取 GitHub Access Token
    let github_token_resp = oauth_client
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

    Ok(user)
}
