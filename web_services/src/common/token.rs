use actix_web::{HttpRequest, dev::ServiceRequest};
use common::{ACCESS_TOKEN, REFRESH_TOKEN};
pub static GITHUB_USER_AGENT: &str = "ani-updater/0.1 (+https://github.com/likanug/ani-updater)";

/// 提取请求中的token
pub trait ExtractToken {
    fn get_access_token(&self) -> Option<String>;
    fn get_refresh_token(&self) -> Option<String>;
}

// 对 HttpRequest 实现
impl ExtractToken for HttpRequest {
    fn get_access_token(&self) -> Option<String> {
        // 1️⃣ 先尝试从 header 读取
        let token_header = self
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer ").map(|s| s.to_string()));
        // 2️⃣ 再尝试从 cookie 读取
        let token_cookie = self.cookie(ACCESS_TOKEN).map(|c| c.value().to_string());
        // 3️⃣ 优先 header，其次 cookie
        token_header.or(token_cookie)
    }

    fn get_refresh_token(&self) -> Option<String> {
        self.cookie(REFRESH_TOKEN).map(|c| c.value().to_string())
    }
}

// 对 ServiceRequest 实现
impl ExtractToken for ServiceRequest {
    fn get_access_token(&self) -> Option<String> {
        // 1️⃣ header
        let token_header = self
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer ").map(|s| s.to_string()));
        // 2️⃣ cookie
        let token_cookie = self.cookie(ACCESS_TOKEN).map(|c| c.value().to_string());

        token_header.or(token_cookie)
    }

    fn get_refresh_token(&self) -> Option<String> {
        self.cookie(REFRESH_TOKEN).map(|c| c.value().to_string())
    }
}
