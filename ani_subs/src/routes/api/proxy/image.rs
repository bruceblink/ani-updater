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
    domain: &'static str,
    referer: &'static str,
    user_agent: &'static str,
}

/// 静态图片源配置
static IMAGE_SOURCES: Lazy<Vec<ImageSource>> = Lazy::new(|| {
    vec![
        ImageSource {
            domain: "hdslb.com",
            referer: "https://www.bilibili.com/",
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Gecko/20100101 Firefox/142.0",
        },
        ImageSource {
            domain: "iqiyipic.com",
            referer: "https://www.iqiyi.com/",
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Gecko/20100101 Firefox/142.0",
        },
    ]
});

#[get("/proxy/image")]
async fn proxy_image(
    web::Query(params): web::Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let url = params
        .get("url")
        .ok_or_else(|| actix_web::error::ErrorBadRequest("缺少 url 参数"))?;

    let parsed =
        reqwest::Url::from_str(url).map_err(|_| actix_web::error::ErrorBadRequest("无效的 url"))?;

    // 只允许 http/https
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return Ok(HttpResponse::BadRequest().body("url scheme 必须是 http 或 https")),
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| actix_web::error::ErrorBadRequest("url 缺少 host"))?;

    let source = IMAGE_SOURCES
        .iter()
        .find(|s| host == s.domain || host.ends_with(&format!(".{}", s.domain)))
        .ok_or_else(|| actix_web::error::ErrorForbidden("host 不允许"))?;

    // 发起上游请求
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
            resp_builder.insert_header((header::CONTENT_TYPE, ct_str));
        }
    } else {
        resp_builder.insert_header((header::CONTENT_TYPE, "image/jpeg")); // 默认兜底
    }

    // Cache-Control
    if let Some(cc) = upstream_resp.headers().get(reqwest::header::CACHE_CONTROL) {
        if let Ok(cc_str) = cc.to_str() {
            resp_builder.insert_header((header::CACHE_CONTROL, cc_str));
        }
    } else {
        resp_builder.insert_header((header::CACHE_CONTROL, "public, max-age=86400"));
    }

    // 跨域头（<img> 一般不需要，但加上更保险）
    resp_builder.insert_header((header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"));

    // 正确的 stream 转换，直接返回 Bytes
    let stream = upstream_resp
        .bytes_stream()
        .map(|chunk_res| chunk_res.map_err(|e| IoError::other(format!("{e}"))));

    Ok(resp_builder.streaming(stream))
}
