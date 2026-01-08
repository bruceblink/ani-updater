use common::api::ApiError;
use common::dto::UserIdentityDto;
use common::utils::AccessToken;
use common::utils::CommonUser;
use common::utils::{GithubUser, RefreshToken, generate_jwt, generate_refresh_token};
use common::{ACCESS_TOKEN, REFRESH_TOKEN};
use infra::Setting;
use infra::UserInfoWithTokenDTO;
use infra::upsert_user_with_third_part;
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
    // 生成系统的refresh_token
    let refresh_token = generate_refresh_token(configuration.token[REFRESH_TOKEN])
        .map_err(|_| ApiError::Internal("refresh_token 生成失败".into()))?;
    // 通过第三方用户创建系统用户
    let user: UserInfoWithTokenDTO = upsert_user_with_third_part(
        &UserIdentityDto {
            provider_user_id: github_user.id.to_string(),
            provider: "github".to_string(),
            email: github_user.clone().email,
            username: github_user.clone().login,
            display_name: github_user.clone().name,
            avatar_url: github_user.clone().avatar_url,
            access_token: None,
            refresh_token: Option::from(refresh_token.token.clone()),
            expires_at: Option::from(refresh_token.clone().expires_at),
        },
        pool,
    )
    .await?;

    // 生成 access_token
    let access_token = generate_jwt(
        &CommonUser {
            id: user.user_id,
            sub: github_user.clone().login,
            uid: github_user.id,
            r#type: "github".to_string(),
            name: Option::from(user.clone().display_name),
            email: Option::from(user.clone().email),
            avatar: user.avatar_url,
            roles: vec![],
            iss: "auth-service".to_string(),
            aud: "api".to_string(),
            ver: 0,
        },
        configuration.token[ACCESS_TOKEN],
    )?;

    Ok((access_token, refresh_token))
}
