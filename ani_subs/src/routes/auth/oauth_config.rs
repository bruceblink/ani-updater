use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};

pub struct OAuthConfig {
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub auth_url: AuthUrl,
    pub token_url: TokenUrl,
    pub redirect_url: RedirectUrl,
}

impl OAuthConfig {
    pub fn from_env() -> Self {
        let client_id =
            ClientId::new(std::env::var("GITHUB_CLIENT_ID").expect("GITHUB_CLIENT_ID 未设置"));
        let client_secret = ClientSecret::new(
            std::env::var("GITHUB_CLIENT_SECRET").expect("GITHUB_CLIENT_SECRET 未设置"),
        );

        let base_url =
            std::env::var("OAUTH_BASE_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());

        let auth_url = AuthUrl::new("https://github.com/login/oauth/authorize".to_string())
            .expect("Invalid authorization endpoint URL");
        let token_url = TokenUrl::new("https://github.com/login/oauth/access_token".to_string())
            .expect("Invalid token endpoint URL");
        let redirect_url = RedirectUrl::new(format!("{base_url}/auth/github/callback"))
            .expect("Invalid redirect URL");

        Self {
            client_id,
            client_secret,
            auth_url,
            token_url,
            redirect_url,
        }
    }
}
