use crate::configuration::Setting;
use anyhow::{Context, Result};
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};

/// 定义 OAuth2 授权的相关配置
#[derive(Clone)]
pub struct OAuthConfig {
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub auth_url: AuthUrl,
    pub token_url: TokenUrl,
    pub redirect_url: RedirectUrl,
    pub jwt_secret: String,
}

impl OAuthConfig {
    pub fn from_configuration(configuration: Setting) -> Result<Self> {
        let client_id = ClientId::new(
            configuration
                .github_client_id
                .context("GITHUB_CLIENT_ID 环境变量未设置")?,
        );

        let client_secret = ClientSecret::new(
            configuration
                .github_client_secret
                .context("GITHUB_CLIENT_SECRET 环境变量未设置")?,
        );

        let base_url = configuration
            .oauth_base_url
            .context("GITHUB_CLIENT_SECRET 环境变量未设置")?;

        let jwt_secret = configuration
            .jwt_secret
            .context("JWT_SECRET 环境变量未设置")?;

        let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
            .context("Invalid auth URL")?;

        let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
            .context("Invalid token URL")?;

        let redirect_url = RedirectUrl::new(format!("{base_url}/auth/github/callback"))
            .context("Invalid redirect URL")?;

        Ok(Self {
            client_id,
            client_secret,
            auth_url,
            token_url,
            redirect_url,
            jwt_secret,
        })
    }
}
