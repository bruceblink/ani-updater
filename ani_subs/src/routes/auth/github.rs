use crate::service::github_user_register;
use actix_web::{HttpRequest, HttpResponse, Responder, cookie::Cookie, get, web};
use common::utils::{GithubUser, generate_jwt, generate_refresh_token, verify_jwt};
use oauth2::{
    AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, Scope, TokenResponse,
    basic::BasicClient,
};
use once_cell::sync::Lazy;
use rand::Rng;
use reqwest::Client;
use sqlx::PgPool;
use std::{collections::HashMap, env, sync::Mutex};

static HTTP: Lazy<Client> = Lazy::new(Client::new);
// 全局内存存储 state -> pkce_verifier
static STATE_PKCE_MAP: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

static USER_AGENT: &str = "ani-updater/0.1 (+https://github.com/likanug/ani-updater)";

static ACCESS_TOKEN: &str = "access_token";
static REFRESH_TOKEN: &str = "refresh_token";

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("访问 /auth/github/login 开始 GitHub 登录")
}

#[get("/me")]
async fn me(req: HttpRequest) -> impl Responder {
    if let Some(cookie) = req.cookie(ACCESS_TOKEN)
        && let Ok(claims) = verify_jwt(cookie.value())
    {
        return HttpResponse::Ok().json(claims);
    }
    HttpResponse::Unauthorized().body("未携带或非法的 JWT")
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
) -> impl Responder {
    let code = query.get("code").cloned().unwrap_or_default();
    let state = query.get("state").cloned().unwrap_or_default();

    let pkce = {
        let mut map = STATE_PKCE_MAP.lock().unwrap();
        map.remove(&state).unwrap_or_default()
    };
    let pkce_verifier = PkceCodeVerifier::new(pkce);

    // 换 token
    let token_res = data
        .get_ref()
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_verifier)
        .request_async(oauth2::reqwest::async_http_client)
        .await;

    let Ok(github_token_resp) = token_res else {
        return HttpResponse::BadRequest().body("换取 github 的 access_token 失败");
    };

    // 拉取 GitHub 用户
    let user_res = HTTP
        .get("https://api.github.com/user")
        .bearer_auth(github_token_resp.access_token().secret())
        .header("User-Agent", USER_AGENT)
        .send()
        .await;

    let Ok(resp) = user_res else {
        return HttpResponse::BadGateway().body("GitHub /user 请求失败");
    };
    let Ok(mut user): Result<GithubUser, _> = resp.json().await else {
        return HttpResponse::BadGateway().body("GitHub 用户信息解析失败");
    };

    // 补充邮箱
    if user.email.is_none()
        && let Ok(resp) = HTTP
            .get("https://api.github.com/user/emails")
            .bearer_auth(github_token_resp.access_token().secret())
            .header("User-Agent", USER_AGENT)
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

    // 生成 assess_token
    let jwt_access = match generate_jwt(&user, 20) {
        Ok(token) => token,
        Err(_) => return HttpResponse::InternalServerError().body("assess_token 生成失败"),
    };

    // 生成 refresh_token
    let jwt_refresh = match generate_refresh_token(15) {
        Ok(token) => token,
        Err(_) => return HttpResponse::InternalServerError().body("refresh_token 生成失败"),
    };

    let frontend_url =
        env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());

    // github 用户注册到当前系统
    let github_access_token = github_token_resp.access_token().secret();
    let refresh_token = match github_user_register(
        pool,
        user,
        Option::from(github_access_token.to_string()),
        jwt_refresh,
    )
    .await
    {
        Ok(token) => token,
        Err(_) => return HttpResponse::InternalServerError().body("refresh_token 持久化失败"),
    };
    // 生成 access_token的cookie
    let access_cookie = Cookie::build(ACCESS_TOKEN, jwt_access)
        .http_only(true)
        .secure(true) // 生产环境必须 https
        .path("/")
        .same_site(actix_web::cookie::SameSite::None) // 为None时可以跨站点请求携带 Cookie
        .finish();
    // 生成 refresh_token的cookie
    let refresh_cookie = Cookie::build(REFRESH_TOKEN, refresh_token)
        .http_only(true)
        .secure(true) // 生产环境必须 https
        .path("/")
        .same_site(actix_web::cookie::SameSite::None) // 为None时可以跨站点请求携带 Cookie
        .finish();
    //设置 HttpOnly Cookie
    HttpResponse::Found()
        .append_header(("Location", frontend_url))
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .finish()
}
/*
/// 刷新 token 接口
#[post("/auth/refresh")]
async fn refresh_token(req: HttpRequest, db: web::Data<PgPool>) -> impl Responder {
    let refresh_cookie = match req.cookie(REFRESH_TOKEN) {
        Some(c) => c,
        None => return HttpResponse::Unauthorized().body("缺少 refresh token"),
    };

    let rec = sqlx::query!(
        "SELECT user_id, expires_at FROM user_identities WHERE refresh_token = $1",
        refresh_cookie.value()
        )
        .fetch_optional(db.get_ref())
        .await
        .unwrap();

    let rec = match rec {
        Some(r) => r,
        None => return HttpResponse::Unauthorized().body("无效 refresh token"),
    };

    if rec.expires_at < Option::from(Utc::now()) {
        return HttpResponse::Unauthorized().body("refresh token 已过期");
    }

    // 生成新的 access_token
    let user = get_user_by_id(db.get_ref(), rec.user_id).await.unwrap();
    let new_access_token = generate_jwt(&user, 20).unwrap();

    // 可选：生成新的 refresh_token，删除旧 token
    let new_refresh_token = create_refresh_token(db.get_ref(), rec.user_id, 30).await.unwrap();
    sqlx::query!("DELETE FROM refresh_tokens WHERE token = $1", refresh_cookie.value())
        .execute(db.get_ref())
        .await
        .unwrap();

    let access_cookie = Cookie::build(ACCESS_TOKEN, new_access_token)
        .http_only(true)
        .secure(true)
        .path("/")
        .same_site(actix_web::cookie::SameSite::Lax)
        .finish();

    let refresh_cookie = Cookie::build(REFRESH_TOKEN, new_refresh_token.token)
        .http_only(true)
        .secure(true)
        .path("/")
        .same_site(actix_web::cookie::SameSite::Lax)
        .finish();

    HttpResponse::Ok()
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .json(serde_json::json!({
            "message": "刷新成功",
            "access_token_exp": chrono::Utc::now().timestamp() + 20 * 60
        }))
}*/
