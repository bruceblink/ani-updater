use actix_web::{HttpResponse, Result, get, http::header, web};
use common::utils::http_client::http_client;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use reqwest::Client;
use std::collections::{HashMap, HashSet};
use std::io::Error as IoError;
use std::str::FromStr;

// 全局 reqwest client（可复用）
static HTTP: Lazy<Client> = Lazy::new(|| http_client().unwrap());

// 可选的域名白名单，防止 SSRF。只允许代理列出的 host
static ALLOWED_HOSTS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut s = HashSet::new();
    s.insert("i0.hdslb.com"); // B站图片 CDN
    s.insert("pic0.iqiyipic.com"); // 爱奇艺图床示例
    s.insert("i2.hdslb.com");
    // 根据需要添加更多 host
    s
});

#[get("/proxy/image")]
async fn image_proxy(
    web::Query(params): web::Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let url = match params.get("url") {
        Some(u) => u,
        None => return Ok(HttpResponse::BadRequest().body("缺少 url 参数")),
    };

    // 简单校验 URL scheme + host（防 SSRF）
    let parsed = match reqwest::Url::from_str(url) {
        Ok(u) => u,
        Err(_) => return Ok(HttpResponse::BadRequest().body("invalid url")),
    };

    // 只允许 http/https
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return Ok(HttpResponse::BadRequest().body("url scheme must be http or https")),
    }

    // 白名单检查：只允许代理可信的图片域名
    if let Some(host) = parsed.host_str() {
        if !ALLOWED_HOSTS.contains(host) {
            return Ok(HttpResponse::Forbidden().body("host not allowed"));
        }
    } else {
        return Ok(HttpResponse::BadRequest().body("url missing host"));
    }

    let upstream_resp = HTTP
        .get(url)
        .send()
        .await
        .map_err(|e| actix_web::error::ErrorBadGateway(format!("请求图片失败: {e}")))?;

    if !upstream_resp.status().is_success() {
        return Ok(HttpResponse::BadGateway().body("上游图片返回错误"));
    }

    let mut resp_builder = HttpResponse::Ok();

    // 复制 Content-Type 头
    if let Some(ct) = upstream_resp.headers().get(reqwest::header::CONTENT_TYPE) {
        if let Ok(ct_str) = ct.to_str() {
            resp_builder.append_header((header::CONTENT_TYPE, ct_str));
        }
    }

    // 复制 Content-Length 头
    if let Some(cl) = upstream_resp.headers().get(reqwest::header::CONTENT_LENGTH) {
        if let Ok(cl_str) = cl.to_str() {
            resp_builder.append_header((header::CONTENT_LENGTH, cl_str));
        }
    }

    // 设置 Cache-Control 头（优先使用上游的，否则使用默认值）
    if let Some(cc) = upstream_resp.headers().get(reqwest::header::CACHE_CONTROL) {
        if let Ok(cc_str) = cc.to_str() {
            resp_builder.append_header((header::CACHE_CONTROL, cc_str));
        }
    } else {
        resp_builder.append_header((header::CACHE_CONTROL, "public, max-age=86400"));
    }

    // 设置跨域头:cite[5]
    resp_builder.append_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"));
    resp_builder.append_header((header::ACCESS_CONTROL_ALLOW_METHODS, "GET, OPTIONS"));
    resp_builder.append_header((header::ACCESS_CONTROL_ALLOW_HEADERS, "Content-Type"));

    // 流式返回图片
    let stream = upstream_resp
        .bytes_stream()
        .map(|chunk_res| match chunk_res {
            Ok(bytes) => Ok::<_, IoError>(bytes),
            Err(e) => Err(std::io::Error::other(format!("{e}"))),
        });

    Ok(resp_builder.streaming(stream))
}
