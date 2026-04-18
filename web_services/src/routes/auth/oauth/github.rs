use crate::common::{AppState, GITHUB_USER_AGENT};
use actix_web::{HttpResponse, cookie::Cookie, get, web};
use common::api::ApiError;
use common::po::ApiResult;
use common::utils::GithubUser;
use common::{ACCESS_TOKEN, REFRESH_TOKEN};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, Scope, TokenResponse,
};
use once_cell::sync::Lazy;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use service::register_service::github_user_register;
use std::collections::HashMap;
use tracing::error;

static HTTP: Lazy<Client> = Lazy::new(Client::new);

/// 允许的 OAuth redirect_uri 白名单。
/// 优先读取 `FRONTEND_DOMAINS` 环境变量（分号分隔的完整 origin，如 `https://dash.likanug.top`）；
/// 若未配置则回退到本地开发默认值。
static ALLOWED_REDIRECT_URIS: Lazy<Vec<String>> = Lazy::new(|| {
    let env_value = std::env::var("FRONTEND_DOMAINS").unwrap_or_default();
    let from_env: Vec<String> = env_value
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if from_env.is_empty() {
        vec![
            "http://localhost:5173".to_string(),
            "http://localhost:3039".to_string(),
        ]
    } else {
        from_env
    }
});

#[derive(Debug, Serialize, Deserialize)]
struct StateClaims {
    redirect_uri: String,
    pkce_verifier: String,
    exp: usize, // UNIX timestamp
}

/// 获取 JWT_SECRET 用于 OAuth state 签名
fn get_jwt_secret() -> Result<String, ApiError> {
    std::env::var("JWT_SECRET").map_err(|_| ApiError::Internal("JWT_SECRET 环境变量未设置".into()))
}

///
///    GitHub 第三方登录的 API <br>
///    /auth/github/login Get 请求 <br>
///    url请求参数:  redirect_uri=xxxx
///
#[get("/auth/oauth/github/login")]
async fn auth_github_login(
    app_state: web::Data<AppState>,
    query: web::Query<HashMap<String, String>>,
) -> ApiResult {
    // 1. 校验 redirect_uri
    let redirect_uri = query
        .get("redirect_uri")
        .ok_or_else(|| ApiError::BadRequest("Invalid redirect_uri".into()))
        .and_then(|uri| validate_redirect_uri(uri))?;

    // 路由仅在 OAuth 已配置时注册，此处 unwrap 安全
    let oauth_client = app_state.oauth_client.as_ref().unwrap();
    let jwt_secret = get_jwt_secret()?;

    // 2. 生成 GitHub 授权地址
    let auth_url = get_github_authorization_url(oauth_client, &redirect_uri, &jwt_secret)
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

///
///    GitHub 第三方登录的回调 API <br>
///    /auth/oauth/github/callback Get 请求 <br>
///    url请求参数:  code=xxxx&state=xxxx
///
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

    let jwt_secret = get_jwt_secret()?;

    // ✅ 严格校验 state.exp
    let mut validation = jsonwebtoken::Validation::default();
    validation.validate_exp = true;

    let token_data = jsonwebtoken::decode::<StateClaims>(
        &state,
        &jsonwebtoken::DecodingKey::from_secret(jwt_secret.as_ref()),
        &validation,
    )
    .map_err(|_| ApiError::Unauthorized("Invalid state".into()))?;

    let redirect_uri = validate_redirect_uri(&token_data.claims.redirect_uri)
        .map_err(|_| ApiError::Unauthorized("Invalid redirect_uri".into()))?;

    let pkce_verifier = PkceCodeVerifier::new(token_data.claims.pkce_verifier);
    // 路由仅在 OAuth 已配置时注册，此处 unwrap 安全
    let oauth_client = app_state.oauth_client.as_ref().unwrap();
    // 换取GitHub access_token
    let github_access_token =
        exchange_github_access_token(oauth_client, code, pkce_verifier).await?;
    // 获取GitHub的用户信息
    let github_user = get_github_user_info(github_access_token).await?;
    // 注册“使用GitHub登录的用户”为系统用户
    let (access_token, refresh_token) =
        github_user_register(&app_state.db_pool, &app_state.configuration, github_user)
            .await
            .map_err(|e| {
                error!("github用户注册为系统用户失败: {e}");
                ApiError::Internal("github用户注册为系统用户失败".into())
            })?;

    let is_prod = app_state.configuration.is_production;
    // 生成 access_token的cookie
    let access_cookie = Cookie::build(ACCESS_TOKEN, access_token.token)
        .http_only(true)
        .secure(is_prod) // 生产环境必须 https
        .path("/")
        .same_site(actix_web::cookie::SameSite::None) // 为None时可以跨站点请求携带 Cookie
        .max_age(time::Duration::minutes(
            app_state.configuration.token[ACCESS_TOKEN],
        ))
        .finish();
    // 生成 refresh_token的cookie
    let refresh_cookie = Cookie::build(REFRESH_TOKEN, refresh_token.token)
        .http_only(true)
        .secure(is_prod)
        .path("/")
        .same_site(actix_web::cookie::SameSite::None)
        .max_age(time::Duration::days(
            app_state.configuration.token[REFRESH_TOKEN],
        ))
        .finish();

    Ok(HttpResponse::Found()
        .append_header(("Location", redirect_uri))
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .finish())
}

fn validate_redirect_uri(input: &str) -> Result<String, ApiError> {
    let redirect_uri = input.trim();
    if redirect_uri.is_empty() {
        return Err(ApiError::BadRequest("Invalid redirect_uri".into()));
    }

    let redirect_url = Url::parse(redirect_uri)
        .map_err(|_| ApiError::BadRequest("Invalid redirect_uri".into()))?;

    let is_allowed = ALLOWED_REDIRECT_URIS.iter().any(|allowed| {
        Url::parse(allowed).ok().is_some_and(|allowed_url| {
            redirect_url.scheme() == allowed_url.scheme()
                && redirect_url.host_str() == allowed_url.host_str()
                && redirect_url.port_or_known_default() == allowed_url.port_or_known_default()
        })
    });

    if !is_allowed {
        return Err(ApiError::BadRequest("Invalid redirect_uri".into()));
    }

    Ok(redirect_url.into())
}

/// 生成 GitHub 授权地址
async fn get_github_authorization_url(
    oauth_client: &BasicClient,
    redirect_uri: &str,
    state_jwt_secret: &str,
) -> anyhow::Result<String> {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let claims = StateClaims {
        redirect_uri: redirect_uri.to_string(),
        pkce_verifier: pkce_verifier.secret().to_string(),
        exp: (chrono::Utc::now().timestamp() + 300) as usize,
    };

    let state_jwt = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(state_jwt_secret.as_ref()),
    )?;

    let (auth_url, _) = oauth_client
        .authorize_url(|| CsrfToken::new(state_jwt))
        .add_scope(Scope::new("read:user".into()))
        .add_scope(Scope::new("user:email".into()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    Ok(auth_url.to_string())
}

/// 换取github的access_token
async fn exchange_github_access_token(
    oauth_client: &BasicClient,
    code: String,
    pkce_verifier: PkceCodeVerifier,
) -> anyhow::Result<String> {
    let github_token_resp = oauth_client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|_| ApiError::Internal("换取 github 的 access_token 失败".into()))?;

    Ok(github_token_resp.access_token().secret().to_string())
}

/// 获取GitHub用户信息
async fn get_github_user_info(access_token: String) -> anyhow::Result<GithubUser> {
    let mut user: GithubUser = HTTP
        .get("https://api.github.com/user")
        .bearer_auth(access_token.clone())
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
            .bearer_auth(access_token)
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
