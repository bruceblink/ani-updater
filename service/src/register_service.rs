use common::dto::UserIdentityDto;
use common::utils::{GithubUser, RefreshToken};
use infra::upsert_user_with_third_part;
use sqlx::PgPool;

/**
  Githu登录的用户注册<br>
  返回refresh_token
*/
pub async fn github_user_register(
    pool: PgPool,
    credentials: GithubUser,
    access_token: Option<String>,
    default_refresh_token: RefreshToken,
) -> anyhow::Result<(i64, String)> {
    let third_part_user = UserIdentityDto {
        provider_user_id: credentials.id.to_string(),
        provider: "github".to_string(),
        email: credentials.email,
        username: credentials.login,
        display_name: credentials.name,
        avatar_url: credentials.avatar_url,
        access_token,
        refresh_token: Option::from(default_refresh_token.token),
        expires_at: Option::from(default_refresh_token.expires_at),
    };
    // 插入数据库,返回refresh_token
    upsert_user_with_third_part(&third_part_user, &pool).await
}
