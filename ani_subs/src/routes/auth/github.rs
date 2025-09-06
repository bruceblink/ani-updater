use crate::common::{ACCESS_TOKEN, ExtractToken, GITHUB_USER_AGENT, REFRESH_TOKEN};
use crate::configuration::Setting;
use crate::service::github_user_register;
use actix_web::{HttpRequest, HttpResponse, Responder, cookie::Cookie, get, post, web};
use common::api::{ApiError, ApiResponse, ApiResult};
use common::utils::{GithubUser, generate_jwt, generate_refresh_token, verify_jwt};
use oauth2::{
    AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, Scope, TokenResponse,
    basic::BasicClient,
};
use once_cell::sync::Lazy;
use rand::Rng;
use reqwest::Client;
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use std::{collections::HashMap, env, sync::Mutex};

static HTTP: Lazy<Client> = Lazy::new(Client::new);
// 全局内存存储 state -> pkce_verifier
static STATE_PKCE_MAP: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[get("/")]
async fn index() -> ApiResult {
    Ok(HttpResponse::Ok().json(ApiResponse::ok("使用 GitHub 进行第三方登录")))
}

#[get("/me")]
async fn me(req: HttpRequest) -> ApiResult {
    if let Some(token) = req.get_access_token()
        && let Ok(claims) = verify_jwt(&token)
    {
        return Ok(HttpResponse::Ok().json(ApiResponse::ok(claims)));
    }
    Err(ApiError::Unauthorized("未携带或非法的 JWT".into()))
}

#[get("/auth/github/login")]
async fn github_login(data: web::Data<BasicClient>) -> impl Responder {
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // 生成随机状态
    let state: String = (0..32)
        .map(|_| {
            let chars = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
            chars[rand::rng().random_range(0..chars.len())] as char
        })
        .collect();

    // 保存 state -> pkce_verifier
    STATE_PKCE_MAP
        .lock()
        .unwrap()
        .insert(state.clone(), pkce_verifier.secret().to_string());

    let (auth_url, _csrf_token) = data
        .get_ref()
        .authorize_url(move || CsrfToken::new(state.clone()))
        .add_scope(Scope::new("read:user".into()))
        .add_scope(Scope::new("user:email".into()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    HttpResponse::Found()
        .append_header(("Location", auth_url.to_string()))
        .finish()
}

#[get("/auth/github/callback")]
async fn github_callback(
    data: web::Data<BasicClient>,
    query: web::Query<HashMap<String, String>>,
    pool: web::Data<PgPool>,
    config: web::Data<Setting>,
) -> ApiResult {
    let code = query.get("code").cloned().unwrap_or_default();
    let state = query.get("state").cloned().unwrap_or_default();

    let pkce = {
        let mut map = STATE_PKCE_MAP.lock().unwrap();
        map.remove(&state).unwrap_or_default()
    };
    let pkce_verifier = PkceCodeVerifier::new(pkce);

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

    let frontend_url =
        env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());

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
    // 使用地址栏传递token
    let final_redirect_url = format!("{frontend_url}/auth/callback?token={}", jwt_access.token);

    Ok(HttpResponse::Found()
        .append_header(("Location", final_redirect_url))
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .json(ApiResponse::ok("登录成功")))
}

#[derive(Debug, Serialize, FromRow)]
struct UserWithIdentity {
    id: i64,
    email: Option<String>,
    username: Option<String>,
    display_name: Option<String>,
    avatar_url: Option<String>,
    provider: Option<String>,
    provider_uid: Option<String>,
}

#[post("/auth/refresh")]
async fn refresh_token(
    req: HttpRequest,
    db: web::Data<PgPool>,
    config: web::Data<Setting>,
) -> ApiResult {
    let old_refresh_cookie = req
        .cookie(REFRESH_TOKEN)
        .ok_or_else(|| ApiError::Unauthorized("缺少 refresh token".into()))?;
    let old_refresh_token = old_refresh_cookie.value();

    let new_refresh_token = generate_refresh_token(config.token[REFRESH_TOKEN] as i64)
        .map_err(|_| ApiError::Internal("refresh_token 生成失败".into()))?;

    let rec = sqlx::query_as::<_, UserWithIdentity>(
        r#"
            WITH valid_token AS (
                SELECT user_id
                FROM refresh_tokens
                WHERE token = $1 AND expires_at > now() AND revoked = false
            ),
            deleted AS (
                DELETE FROM refresh_tokens
                WHERE token = $1
                RETURNING user_id
            ),
            inserted AS (
                INSERT INTO refresh_tokens (user_id, token, expires_at)
                SELECT user_id, $2, now() + interval '30 days'
                FROM valid_token
                RETURNING user_id
            )
            SELECT ui.id, ui.email, ui.username, ui.display_name, ui.avatar_url,
                   uident.provider, uident.provider_uid
            FROM user_info ui
            JOIN valid_token vt ON ui.id = vt.user_id
            LEFT JOIN user_identities uident ON ui.id = uident.user_id
        "#,
    )
    .bind(old_refresh_token)
    .bind(&new_refresh_token.token)
    .fetch_optional(db.get_ref())
    .await
    .map_err(|e| {
        tracing::error!("刷新 token 查询失败: {e}");
        ApiError::Internal("服务器错误".into())
    })?;

    let github_user = match rec {
        Some(u) => GithubUser {
            login: u.username.unwrap_or_default(),
            id: u.provider_uid.unwrap_or_default().parse().unwrap_or(0),
            avatar_url: u.avatar_url,
            name: u.display_name,
            email: u.email,
        },
        None => return Err(ApiError::Unauthorized("refresh token 无效或已过期".into())),
    };

    let new_access_token = generate_jwt(&github_user, config.token[ACCESS_TOKEN] as i64)
        .map_err(|_| ApiError::Unauthorized("refresh token 无效或已过期".into()))?;

    let access_cookie = Cookie::build(ACCESS_TOKEN, new_access_token.token.clone())
        .http_only(true)
        .secure(true)
        .path("/")
        .same_site(actix_web::cookie::SameSite::None)
        .finish();

    let refresh_cookie = Cookie::build(REFRESH_TOKEN, new_refresh_token.token.clone())
        .http_only(true)
        .secure(true)
        .path("/")
        .same_site(actix_web::cookie::SameSite::None)
        .finish();

    Ok(HttpResponse::Ok()
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .json(ApiResponse::ok(serde_json::json!({
            "message": "刷新成功",
            "access_token_exp": new_access_token.expires_at.timestamp() as usize,
            "user": github_user
        }))))
}
