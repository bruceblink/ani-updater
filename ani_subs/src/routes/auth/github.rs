use actix_web::{HttpRequest, HttpResponse, Responder, cookie::Cookie, get, web};
use common::utils::{GithubUser, generate_jwt, verify_jwt};
use oauth2::{
    AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, Scope, TokenResponse,
    basic::BasicClient,
};
use once_cell::sync::Lazy;
use rand::Rng;
use reqwest::Client;
use std::{collections::HashMap, env, sync::Mutex};

static HTTP: Lazy<Client> = Lazy::new(Client::new);
// 全局内存存储 state -> pkce_verifier
static STATE_PKCE_MAP: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("访问 /auth/github/login 开始 GitHub 登录")
}

#[get("/me")]
async fn me(req: HttpRequest) -> impl Responder {
    if let Some(cookie) = req.cookie("access_token")
        && let Some(claims) = verify_jwt(cookie.value())
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

    let Ok(token) = token_res else {
        return HttpResponse::BadRequest().body("换取 access_token 失败");
    };

    // 拉取 GitHub 用户
    let user_res = HTTP
        .get("https://api.github.com/user")
        .bearer_auth(token.access_token().secret())
        .header("User-Agent", "actix-github-oauth-demo")
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
            .bearer_auth(token.access_token().secret())
            .header("User-Agent", "actix-github-oauth-demo")
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

    // 生成 JWT 并设置 HttpOnly Cookie
    let jwt = generate_jwt(&user, 20); // 20分钟过期
    let frontend_url =
        env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());

    let cookie = Cookie::build("access_token", jwt)
        .http_only(true)
        .secure(true) // 生产环境必须 https
        .path("/")
        .same_site(actix_web::cookie::SameSite::Lax)
        .finish();

    HttpResponse::Found()
        .append_header(("Location", frontend_url))
        .cookie(cookie)
        .finish()
}

/// 刷新 token 接口
#[get("/auth/refresh")]
async fn refresh_token(req: HttpRequest) -> impl Responder {
    if let Some(cookie) = req.cookie("access_token")
        && let Some(claims) = verify_jwt(cookie.value())
    {
        // 生成新的 JWT
        let user = GithubUser {
            login: claims.sub,
            id: claims.uid,
            avatar_url: None,
            name: claims.name,
            email: claims.email,
        };
        let new_jwt = generate_jwt(&user, 2);

        let new_cookie = Cookie::build("access_token", new_jwt)
            .http_only(true)
            .secure(true)
            .path("/")
            .same_site(actix_web::cookie::SameSite::Lax)
            .finish();

        return HttpResponse::Ok().cookie(new_cookie).body("刷新成功");
    }
    HttpResponse::Unauthorized().body("无效 token")
}
