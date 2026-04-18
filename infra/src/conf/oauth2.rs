use crate::conf::configuration::Setting;
use anyhow::{Context, Result};
use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use secrecy::ExposeSecret;
use tracing::warn;

/// 定义 GitHub OAuth2 授权所需的配置（不含 jwt_secret，jwt_secret 由 JWT_SECRET 环境变量独立加载）
#[derive(Clone)]
pub struct OAuthConfig {
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub auth_url: AuthUrl,
    pub token_url: TokenUrl,
    pub redirect_url: RedirectUrl,
}

impl OAuthConfig {
    /// 尝试从配置中构建 OAuthConfig。
    /// 若 GITHUB_CLIENT_ID / GITHUB_CLIENT_SECRET / OAUTH_BASE_URL 任一未配置，则返回 None
    /// 并输出警告日志；应用仍可正常启动，但 GitHub OAuth 登录路由不会注册。
    pub fn try_from_configuration(configuration: &Setting) -> Result<Option<Self>> {
        let (Some(client_id_str), Some(client_secret), Some(base_url)) = (
            configuration.github_client_id.as_ref(),
            configuration.github_client_secret.as_ref(),
            configuration.oauth_base_url.as_ref(),
        ) else {
            warn!(
                "GitHub OAuth 未配置（缺少 GITHUB_CLIENT_ID / GITHUB_CLIENT_SECRET / OAUTH_BASE_URL），\
                GitHub 登录功能将被禁用"
            );
            return Ok(None);
        };

        let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
            .context("GitHub auth URL 无效")?;

        let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
            .context("GitHub token URL 无效")?;

        let redirect_url =
            RedirectUrl::new(format!("{base_url}/auth/oauth/github/callback"))
                .context("OAUTH_BASE_URL 格式无效，无法构造 redirect URL")?;

        Ok(Some(Self {
            client_id: ClientId::new(client_id_str.clone()),
            client_secret: ClientSecret::new(
                client_secret.expose_secret().clone(),
            ),
            auth_url,
            token_url,
            redirect_url,
        }))
    }
}

/// 尝试创建 OAuth 配置，未配置时返回 None
pub fn try_create_oauth_config(configuration: &Setting) -> Result<Option<OAuthConfig>> {
    OAuthConfig::try_from_configuration(configuration)
}

/// 创建 OAuth 客户端
pub fn create_oauth_client(config: &OAuthConfig) -> Result<BasicClient> {
    let client = BasicClient::new(
        config.client_id.clone(),
        Some(config.client_secret.clone()),
        config.auth_url.clone(),
        Some(config.token_url.clone()),
    )
    .set_redirect_uri(config.redirect_url.clone());

    Ok(client)
}
