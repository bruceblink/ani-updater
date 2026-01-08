use common::dto::UserDto;
use common::utils::AccessToken;
use common::utils::CommonUser;
use common::utils::{GithubUser, RefreshToken, generate_jwt, generate_refresh_token};
use common::{ACCESS_TOKEN, REFRESH_TOKEN};
use infra::Setting;
use sqlx::PgPool;

/**
  Githu登录的用户注册<br>
  返回refresh_token
*/
pub async fn github_user_register(
    pool: &PgPool,
    configuration: &Setting,
    github_user: GithubUser,
) -> anyhow::Result<(AccessToken, RefreshToken)> {
    let mut tx = pool.begin().await?;

    // 1. 用户身份（只管 user）
    let user = find_or_create_user_by_github(&mut tx, &github_user).await?;

    // 2. 生成 refresh_token
    let refresh = generate_refresh_token(configuration.token[REFRESH_TOKEN])?;

    // 3. refresh_token 入库
    insert_refresh_token(&mut tx, user.id, &refresh).await?;

    // 4. 生成 access_token
    let access = generate_jwt(
        &CommonUser {
            id: user.id,
            sub: github_user.login,
            uid: github_user.id,
            r#type: "github".to_string(),
            name: Option::from(user.display_name),
            email: Option::from(user.email),
            avatar: Option::from(user.avatar_url),
            roles: vec![],
            iss: "auth-service".to_string(),
            aud: "api".to_string(),
            ver: 0,
        },
        configuration.token[ACCESS_TOKEN],
    )?;

    tx.commit().await?;

    Ok((access, refresh))
}

/// Function to find or create a user based on GitHub information
pub async fn find_or_create_user_by_github(
    _tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    _github_user: &GithubUser,
) -> anyhow::Result<UserDto> {
    todo!()
}

/// Insert the refresh token into the database
async fn insert_refresh_token(
    _tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    _user_id: i64,
    _refresh_token: &RefreshToken,
) -> anyhow::Result<()> {
    todo!()
}
