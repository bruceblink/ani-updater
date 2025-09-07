use actix_web::{HttpResponse, Result, get, http::header, web};
use common::utils::http_client::http_client;
use futures_util::StreamExt;
use once_cell::sync::Lazy;
use reqwest::Client;
use std::collections::HashMap;
use std::io::Error as IoError;
use std::str::FromStr;

// 全局 reqwest client（可复用）
static HTTP: Lazy<Client> = Lazy::new(|| http_client().unwrap());

/// 定义图片源信息
struct ImageSource {
    host: &'static str,
    referer: &'static str,
    user_agent: &'static str,
}

/// 静态图片源配置
static IMAGE_SOURCES: Lazy<Vec<ImageSource>> = Lazy::new(|| {
    vec![
        ImageSource {
            host: "hdslb.com",
            referer: "https://www.bilibili.com/",
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Gecko/20100101 Firefox/142.0",
        },
        ImageSource {
            host: "iqiyipic.com",
            referer: "https://www.iqiyi.com/",
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Gecko/20100101 Firefox/142.0",
        },
    ]
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

    // 严格检查 host
    let source = match parsed.host_str() {
        Some(host) => IMAGE_SOURCES.iter().find(|s| host.contains(s.host)),
        None => None,
    };

    let source = match source {
        Some(s) => s,
        None => return Ok(HttpResponse::Forbidden().body("host 不允许")),
    };

    // 发起上游请求，带 Referer 和 User-Agent
    let upstream_resp = HTTP
        .get(url)
        .header("Referer", source.referer)
        .header("User-Agent", source.user_agent)
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

    // Cache-Control 优先使用上游，否则默认 1 天
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
