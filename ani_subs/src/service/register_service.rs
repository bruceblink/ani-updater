use crate::dao::upsert_user_with_third_part;
use crate::domain::dto::UserIdentityDto;
use actix_web::web;
use common::utils::GithubUser;

pub async fn github_user_register(
    pool: web::Data<sqlx::PgPool>,
    credentials: GithubUser,
    access_token: Option<String>,
) -> anyhow::Result<()> {
    let third_part_user = UserIdentityDto {
        provider_user_id: credentials.id.to_string(),
        provider: "github".to_string(),
        email: credentials.email,
        username: credentials.login,
        display_name: credentials.name,
        avatar_url: credentials.avatar_url,
        access_token,
    };
    upsert_user_with_third_part(&third_part_user, &pool).await
}
