use common::api::ApiResponse;
use common::po::{HealthItem, ItemResult, TaskItem};
use common::utils::date_utils::get_today_weekday;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::collections::HashSet;

/// 全局共享的 HTTP client（内部已 Arc 化，clone() 代价极低）
pub(crate) static HTTP_CLIENT: Lazy<Client> = Lazy::new(Client::new);

/// 通用：POST 空请求体到 api_url，返回响应的 JSON Value
async fn post_and_get_json(api_url: &str) -> Result<serde_json::Value, String> {
    let response = HTTP_CLIENT
        .post(api_url)
        .header("Referer", api_url)
        .header("Content-Type", "application/json")
        .json(&json!({}))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!(
            "HTTP错误: {} - 状态码: {}",
            api_url,
            response.status()
        ));
    }

    response.json().await.map_err(|e| e.to_string())
}

/// 通用：构建单条 TaskItem 的 ItemResult
fn build_single_item_result(weekday: String, item: TaskItem) -> ItemResult {
    let mut all_items = HashSet::new();
    all_items.insert(item);
    let mut result = HashMap::new();
    result.insert(weekday, all_items);
    result
}

pub async fn extract_news_item(api_url: String) -> anyhow::Result<ApiResponse<ItemResult>, String> {
    let json_value = post_and_get_json(&api_url).await?;
    let weekday = get_today_weekday().name_cn.to_string();
    let item = TaskItem::ExtractNewsItem(HealthItem { url: api_url, result: json_value });
    Ok(ApiResponse::ok(build_single_item_result(weekday, item)))
}

/// 提取新增关键字到 news_keywords 表
pub async fn extract_news_keywords(api_url: String) -> anyhow::Result<ApiResponse<ItemResult>, String> {
    let json_value = post_and_get_json(&api_url).await?;
    let weekday = get_today_weekday().name_cn.to_string();
    let item = TaskItem::ExtractNewsNewsKeywords(HealthItem { url: api_url, result: json_value });
    Ok(ApiResponse::ok(build_single_item_result(weekday, item)))
}

pub async fn extract_news_event(api_url: String) -> anyhow::Result<ApiResponse<ItemResult>, String> {
    let json_value = post_and_get_json(&api_url).await?;
    let weekday = get_today_weekday().name_cn.to_string();
    let item = TaskItem::ExtractNewsEvent(HealthItem { url: api_url, result: json_value });
    Ok(ApiResponse::ok(build_single_item_result(weekday, item)))
}

pub async fn merge_cross_day_news_events(api_url: String) -> anyhow::Result<ApiResponse<ItemResult>, String> {
    let json_value = post_and_get_json(&api_url).await?;
    let weekday = get_today_weekday().name_cn.to_string();
    let item = TaskItem::MergeNewsItem(HealthItem { url: api_url, result: json_value });
    Ok(ApiResponse::ok(build_single_item_result(weekday, item)))
}
