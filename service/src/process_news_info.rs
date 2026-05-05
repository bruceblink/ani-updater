use common::api::ApiResponse;
use common::po::{HealthItem, ItemResult, NewsInfo, TaskItem};
use common::utils::date_utils::get_today_weekday;
use once_cell::sync::Lazy;
use reqwest::{Client, Url};
use serde_json::json;
use std::collections::HashMap;
use std::collections::HashSet;

/// 全局共享的 HTTP client（内部已 Arc 化，clone() 代价极低）
pub(crate) static HTTP_CLIENT: Lazy<Client> = Lazy::new(Client::new);

/// 通用：POST 空请求体到 api_url，返回响应的 JSON Value
async fn post_and_get_json(api_url: &str) -> Result<serde_json::Value, String> {
    let parsed_url = Url::parse(api_url).map_err(|e| format!("非法URL: {api_url}, error: {e}"))?;
    let response = HTTP_CLIENT
        .post(parsed_url.clone())
        .header("Referer", parsed_url.as_str())
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
    let item = TaskItem::ExtractNewsItem(HealthItem {
        url: api_url,
        result: json_value,
    });
    Ok(ApiResponse::ok(build_single_item_result(weekday, item)))
}

/// 提取新增关键字到 news_keywords 表
pub async fn extract_news_keywords(
    api_url: String,
) -> anyhow::Result<ApiResponse<ItemResult>, String> {
    let json_value = post_and_get_json(&api_url).await?;
    let weekday = get_today_weekday().name_cn.to_string();
    let item = TaskItem::ExtractNewsNewsKeywords(HealthItem {
        url: api_url,
        result: json_value,
    });
    Ok(ApiResponse::ok(build_single_item_result(weekday, item)))
}

pub async fn extract_news_event(
    api_url: String,
) -> anyhow::Result<ApiResponse<ItemResult>, String> {
    let json_value = post_and_get_json(&api_url).await?;
    let weekday = get_today_weekday().name_cn.to_string();
    let item = TaskItem::ExtractNewsEvent(HealthItem {
        url: api_url,
        result: json_value,
    });
    Ok(ApiResponse::ok(build_single_item_result(weekday, item)))
}

pub async fn merge_cross_day_news_events(
    api_url: String,
) -> anyhow::Result<ApiResponse<ItemResult>, String> {
    let json_value = post_and_get_json(&api_url).await?;
    let weekday = get_today_weekday().name_cn.to_string();
    let item = TaskItem::MergeNewsItem(HealthItem {
        url: api_url,
        result: json_value,
    });
    Ok(ApiResponse::ok(build_single_item_result(weekday, item)))
}

/// 从 latestnews 抓取所有新闻源数据，存入 news_info 表
/// base_url 格式: https://news.likanug.top
/// 步骤：1. GET /api/s/ids 获取所有 sourceId
///       2. 并发 GET /api/s?id={sourceId} 获取每个源数据
pub async fn fetch_all_news(base_url: String) -> anyhow::Result<ApiResponse<ItemResult>, String> {
    let ids_url = format!("{base_url}/api/s/ids");
    let ids: Vec<String> = HTTP_CLIENT
        .get(&ids_url)
        .send()
        .await
        .map_err(|e| format!("获取 ids 失败: {e}"))?
        .json()
        .await
        .map_err(|e| format!("解析 ids 响应失败: {e}"))?;

    let weekday = get_today_weekday().name_cn.to_string();
    let mut all_items: HashSet<TaskItem> = HashSet::new();

    // 并发抓取所有源
    let tasks: Vec<_> = ids
        .into_iter()
        .map(|id| {
            let url = format!("{base_url}/api/s?id={id}");
            let client = HTTP_CLIENT.clone();
            tokio::spawn(async move {
                let resp = client.get(&url).send().await.ok()?;
                if !resp.status().is_success() {
                    return None;
                }
                let json_value: serde_json::Value = resp.json().await.ok()?;
                let source_id = json_value["id"].as_str()?.to_string();
                let name = json_value["name"]
                    .as_str()
                    .unwrap_or(&source_id)
                    .to_string();
                let items = json_value["items"].as_array().cloned().unwrap_or_default();
                Some(NewsInfo {
                    id: source_id,
                    name,
                    items,
                })
            })
        })
        .collect();

    for task in tasks {
        if let Ok(Some(news)) = task.await {
            all_items.insert(TaskItem::News(news));
        }
    }

    let mut result = HashMap::new();
    result.insert(weekday, all_items);
    Ok(ApiResponse::ok(result))
}
