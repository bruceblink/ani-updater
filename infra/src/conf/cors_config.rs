use actix_cors::Cors;
use actix_web::http::header;

/// 配置 CORS 中间件
pub fn configure_cors(allowed_origins: Vec<String>) -> Cors {
    if allowed_origins.is_empty() {
        // 如果没有配置允许的源，使用默认设置（仅允许同源）
        Cors::default()
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
            .supports_credentials()
    } else {
        // 根据配置的源列表设置 CORS
        // 将 allowed_origins 移动到闭包中
        Cors::default()
            .allowed_origin_fn(move |origin, _req_head| {
                allowed_origins
                    .iter()
                    .any(|o| origin.as_bytes() == o.as_bytes())
            })
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
            .supports_credentials()
    }
}
