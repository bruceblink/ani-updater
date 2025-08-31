use actix_web::{HttpRequest, HttpResponse, Responder, get, web};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use oauth2::{
    AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, Scope, TokenResponse,
    basic::BasicClient,
};
use once_cell::sync::Lazy;
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Mutex;

static HTTP: Lazy<Client> = Lazy::new(Client::new);
// 全局内存存储 state -> pkce_verifier
static STATE_PKCE_MAP: Lazy<Mutex<std::collections::HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(std::collections::HashMap::new()));

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // GitHub login
    uid: u64,    // GitHub ID
    exp: usize,  // 过期时间戳
    name: Option<String>,
    email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GhUser {
    login: String,
    id: u64,
    avatar_url: Option<String>,
    name: Option<String>,
    email: Option<String>,
}

/// 生成 JWT
fn generate_jwt(user: &GhUser) -> String {
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET 未设置");
    let exp = chrono::Utc::now() + chrono::Duration::hours(2);

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

/// 校验 JWT
fn verify_jwt(token: &str) -> Option<Claims> {
    let secret = env::var("JWT_SECRET").ok()?;
    let decoded = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )
    .ok()?;
    Some(decoded.claims)
}

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok().body("访问 /auth/github/login 开始 GitHub 登录")
}

#[get("/me")]
async fn me(req: HttpRequest) -> impl Responder {
    let auth = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    if let Some(auth_header) = auth {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            if let Some(claims) = verify_jwt(token) {
                return HttpResponse::Ok().json(claims);
            }
        }
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
            chars[rand::thread_rng().gen_range(0..chars.len())] as char
        })
        .collect();

    // 保存 state -> pkce_verifier
    STATE_PKCE_MAP
        .lock()
        .unwrap()
        .insert(state.clone(), pkce_verifier.secret().to_string());

    //let csrf_token = CsrfToken::new(state.clone());

    let auth_url = data
        .get_ref()
        .authorize_url(move || CsrfToken::new(state.clone()))
        .add_scope(Scope::new("read:user".into()))
        .add_scope(Scope::new("user:email".into()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    HttpResponse::Found()
        .append_header(("Location", auth_url.0.to_string()))
        .finish()
}

#[get("/auth/github/callback")]
async fn github_callback(
    data: web::Data<BasicClient>,
    query: web::Query<serde_json::Value>,
) -> impl Responder {
    let code = query.get("code").and_then(|v| v.as_str()).unwrap_or("");
    let state = query.get("state").and_then(|v| v.as_str()).unwrap_or("");
    // let pkce = query.get("pkce").and_then(|v| v.as_str()).unwrap_or("");

    // 用 state 查找 pkce_verifier
    let pkce = {
        let mut map = STATE_PKCE_MAP.lock().unwrap();
        map.remove(state).unwrap_or_default()
    };

    // PKCE verifier
    let pkce_verifier = PkceCodeVerifier::new(pkce.to_string());

    // 换 token
    let token_res = data
        .get_ref()
        .exchange_code(AuthorizationCode::new(code.to_string()))
        .set_pkce_verifier(pkce_verifier)
        .request_async(oauth2::reqwest::async_http_client)
        .await;

    let Ok(token) = token_res else {
        return HttpResponse::BadRequest()
            .content_type("text/plain; charset=utf-8")
            .body("换取 access_token 失败");
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
    let Ok(mut user): Result<GhUser, _> = resp.json().await else {
        return HttpResponse::BadGateway().body("GitHub 用户信息解析失败");
    };

    // 邮箱补充
    if user.email.is_none() {
        let emails_res = HTTP
            .get("https://api.github.com/user/emails")
            .bearer_auth(token.access_token().secret())
            .header("User-Agent", "actix-github-oauth-demo")
            .send()
            .await;

        if let Ok(resp) = emails_res {
            if let Ok(emails) = resp.json::<Vec<serde_json::Value>>().await {
                if let Some(email) = emails.first() {
                    user.email = email
                        .get("email")
                        .and_then(|e| e.as_str())
                        .map(String::from);
                }
            }
        }
    }

    // 生成 JWT
    let jwt = generate_jwt(&user);

    // 重定向到前端
    let frontend_url =
        env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let url = format!("{frontend_url}?token={jwt}");

    HttpResponse::Found()
        .append_header(("Location", url))
        .finish()
}
