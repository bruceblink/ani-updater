use crate::common::{AppState, ExtractToken};
use actix_web::cookie::{Cookie, SameSite};
use actix_web::{HttpRequest, HttpResponse, Responder, post, web};

#[post("/logout")]
async fn logout(app_state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
    // 1️⃣ 注销 refresh_token（数据库标记为 revoked）
    if let Some(refresh_token) = req.get_refresh_token() {
        if let Err(e) = sqlx::query(
            r#"
                UPDATE refresh_tokens SET revoked = true WHERE token = $1;
            "#,
        )
        .bind(refresh_token.clone())
        .execute(&app_state.db_pool)
        .await
        {
            tracing::error!("注销 refresh_token {refresh_token:?} 失败: {e}");
        }
    } else {
        tracing::warn!("用户登出时未携带 refresh_token");
    }

    // 2️⃣ 清空 access_token 和 refresh_token cookie
    let is_prod = app_state.configuration.is_production;
    let expired_cookie = |name: String| {
        Cookie::build(name, "")
            .path("/")
            .http_only(true)
            .secure(is_prod)
            .same_site(SameSite::None)
            .expires(time::OffsetDateTime::now_utc() - time::Duration::seconds(1))
            .finish()
    };

    let access_cookie = expired_cookie("access_token".to_string());
    let refresh_cookie = expired_cookie("refresh_token".to_string());

    // 3️⃣ 返回 204 No Content 更语义化
    HttpResponse::NoContent()
        .cookie(access_cookie)
        .cookie(refresh_cookie)
        .finish()
}
