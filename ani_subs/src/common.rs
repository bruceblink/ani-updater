use actix_web::{HttpRequest, dev::ServiceRequest};

pub const ACCESS_TOKEN: &str = "access_token";
pub const REFRESH_TOKEN: &str = "refresh_token";

/// 提取请求中的token
pub trait ExtractToken {
    fn get_access_token(&self) -> Option<String>;
    fn get_refresh_token(&self) -> Option<String>;
}

// 对 HttpRequest 实现
impl ExtractToken for HttpRequest {
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
