use crate::common::AppState;
use actix_web::{HttpResponse, Responder, post, web};
use common::api::ApiResponse;
use common::utils::{CommonUser, generate_jwt, generate_refresh_token};
use common::{ACCESS_TOKEN, REFRESH_TOKEN};
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,   // 用户名
    pub password: String,   // 密码
    pub email: Option<String>, // 邮箱（可选）
}

#[derive(Serialize)]
struct RegisterResponse {
    user_id: i64,
    username: String,
}

#[post("/register")]
pub async fn register(
    app_state: web::Data<AppState>,
    body: web::Json<RegisterRequest>,
) -> impl Responder {
    let username = body.username.trim();
    let password = body.password.as_str();

    // 基本输入校验
    if username.is_empty() {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::err("用户名不能为空"));
    }
    if password.len() < 8 {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::err("密码长度不能少于 8 位"));
    }

    // 对密码进行 bcrypt 哈希
    let password_hash = match bcrypt::hash(password, bcrypt::DEFAULT_COST) {
        Ok(h) => h,
        Err(e) => {
            error!("密码哈希失败: {e}");
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::err("服务器内部错误"));
        }
    };

    let email = body.email.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());

    // 在事务中插入用户并分配默认角色
    let result = async {
        let mut tx = app_state.db_pool.begin().await?;

        // 插入用户
        let user_id: i64 = sqlx::query_scalar(
            r#"
            INSERT INTO user_info (username, password, email)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
        )
        .bind(username)
        .bind(&password_hash)
        .bind(email)
        .fetch_one(&mut *tx)
        .await?;

        // 分配 user 角色
        sqlx::query(
            r#"
            INSERT INTO user_roles (user_id, role_id)
            SELECT $1, id FROM roles WHERE name = 'user'
            "#,
        )
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok::<i64, sqlx::Error>(user_id)
    }
    .await;

    match result {
        Ok(user_id) => {
            // 生成 access_token
            let roles = vec!["user".to_string()];
            let common_user = CommonUser {
                id: user_id,
                sub: username.to_string(),
                uid: user_id,
                email: email.map(|s| s.to_string()),
                avatar_url: None,
                r#type: "local".to_string(),
                roles,
                ver: 0,
            };
            let access_token_mins = app_state.configuration.token[ACCESS_TOKEN];
            let refresh_token_days = app_state.configuration.token[REFRESH_TOKEN];

            let access_token = match generate_jwt(&common_user, access_token_mins) {
                Ok(t) => t,
                Err(e) => {
                    error!("生成 access_token 失败: {e}");
                    return HttpResponse::InternalServerError()
                        .json(ApiResponse::<()>::err("服务器内部错误"));
                }
            };
            let refresh_token = match generate_refresh_token(refresh_token_days) {
                Ok(t) => t,
                Err(e) => {
                    error!("生成 refresh_token 失败: {e}");
                    return HttpResponse::InternalServerError()
                        .json(ApiResponse::<()>::err("服务器内部错误"));
                }
            };

            // 持久化 refresh_token
            let persist_result = sqlx::query(
                r#"
                INSERT INTO refresh_tokens (user_id, token, expires_at, session_expires_at)
                VALUES ($1, $2, $3, $4)
                "#,
            )
            .bind(user_id)
            .bind(&refresh_token.token)
            .bind(refresh_token.expires_at)
            .bind(refresh_token.expires_at) // session 与 token 同周期
            .execute(&app_state.db_pool)
            .await;

            if let Err(e) = persist_result {
                error!("持久化 refresh_token 失败: {e}");
                return HttpResponse::InternalServerError()
                    .json(ApiResponse::<()>::err("服务器内部错误"));
            }

            let is_prod = app_state.configuration.is_production;
            let access_cookie = actix_web::cookie::Cookie::build(ACCESS_TOKEN, access_token.token)
                .http_only(true)
                .secure(is_prod)
                .path("/")
                .same_site(actix_web::cookie::SameSite::None)
                .max_age(time::Duration::minutes(access_token_mins))
                .finish();
            let refresh_cookie =
                actix_web::cookie::Cookie::build(REFRESH_TOKEN, refresh_token.token)
                    .http_only(true)
                    .secure(is_prod)
                    .path("/")
                    .same_site(actix_web::cookie::SameSite::None)
                    .max_age(time::Duration::days(refresh_token_days))
                    .finish();

            HttpResponse::Created()
                .cookie(access_cookie)
                .cookie(refresh_cookie)
                .json(ApiResponse::ok(RegisterResponse {
                    user_id,
                    username: username.to_string(),
                }))
        }
        Err(e) => {
            // 唯一约束冲突（用户名/邮箱已存在）
            let msg = e.to_string();
            if msg.contains("unique") || msg.contains("duplicate") {
                return HttpResponse::Conflict()
                    .json(ApiResponse::<()>::err("用户名或邮箱已被注册"));
            }
            error!("注册失败: {e}");
            HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::err("服务器内部错误"))
        }
    }
}

