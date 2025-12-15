use common::api::ApiResponse;
use common::po::{HealthItem, ItemResult, TaskItem};
use common::utils::date_utils::get_today_weekday;
use serde_json::json;
use std::collections::HashMap;

pub async fn extract_news_item(api_url: String) -> anyhow::Result<ApiResponse<ItemResult>, String> {
    let mut result: ItemResult = HashMap::new();
    let client = reqwest::Client::new();
    let weekday = get_today_weekday().name_cn.to_string();
    //  构建空的请求体 JSON 数据，使用默认请求参数
    let request_body = json!({});
    let response = client
        .post(api_url.clone())
        .header("Referer", api_url.clone())
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // 检查HTTP状态码
    if !response.status().is_success() {
        return Err(format!(
            "HTTP错误: {} - 状态码: {}",
            api_url.clone(),
            response.status()
        ));
    }
    // 将响应解析成json
    let json_value: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    let all_items: Vec<TaskItem> = vec![TaskItem::ExtractNewsItem(HealthItem {
        url: api_url.to_string(),
        result: json_value,
    })];
    result.insert(weekday, all_items);
    Ok(ApiResponse::ok(result))
}

/// 提取新增关键字到news_keywords表
pub async fn extract_news_keywords(
    api_url: String,
) -> anyhow::Result<ApiResponse<ItemResult>, String> {
    let mut result: ItemResult = HashMap::new();
    let client = reqwest::Client::new();
    let weekday = get_today_weekday().name_cn.to_string();
    //  构建空的请求体 JSON 数据，使用默认请求参数
    let request_body = json!({});
    let response = client
        .post(api_url.clone())
        .header("Referer", api_url.clone())
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // 检查HTTP状态码
    if !response.status().is_success() {
        return Err(format!(
            "HTTP错误: {} - 状态码: {}",
            api_url.clone(),
            response.status()
        ));
    }
    // 将响应解析成json
    let json_value: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    let all_items: Vec<TaskItem> = vec![TaskItem::ExtractNewsNewsKeywords(HealthItem {
        url: api_url.to_string(),
        result: json_value,
    })];
    result.insert(weekday, all_items);
    Ok(ApiResponse::ok(result))
}

pub async fn extract_news_event(
    api_url: String,
) -> anyhow::Result<ApiResponse<ItemResult>, String> {
    let mut result: ItemResult = HashMap::new();
    let client = reqwest::Client::new();
    let weekday = get_today_weekday().name_cn.to_string();
    //  构建空的请求体 JSON 数据，使用默认请求参数
    let request_body = json!({});
    let response = client
        .post(api_url.clone())
        .header("Referer", api_url.clone())
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // 检查HTTP状态码
    if !response.status().is_success() {
        return Err(format!(
            "HTTP错误: {} - 状态码: {}",
            api_url.clone(),
            response.status()
        ));
    }
    // 将响应解析成json
    let json_value: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    let all_items: Vec<TaskItem> = vec![TaskItem::ExtractNewsEvent(HealthItem {
        url: api_url.to_string(),
        result: json_value,
    })];
    result.insert(weekday, all_items);
    Ok(ApiResponse::ok(result))
}

pub async fn merge_cross_day_news_events(
    api_url: String,
) -> anyhow::Result<ApiResponse<ItemResult>, String> {
    let mut result: ItemResult = HashMap::new();
    let client = reqwest::Client::new();
    let weekday = get_today_weekday().name_cn.to_string();
    //  构建空的请求体 JSON 数据，使用默认请求参数
    let request_body = json!({});
    let response = client
        .post(api_url.clone())
        .header("Referer", api_url.clone())
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // 检查HTTP状态码
    if !response.status().is_success() {
        return Err(format!(
            "HTTP错误: {} - 状态码: {}",
            api_url.clone(),
            response.status()
        ));
    }
    // 将响应解析成json
    let json_value: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;

    let all_items: Vec<TaskItem> = vec![TaskItem::MergeNewsItem(HealthItem {
        url: api_url.to_string(),
        result: json_value,
    })];
    result.insert(weekday, all_items);
    Ok(ApiResponse::ok(result))
}

#[cfg(test)]
mod test {

    #[tokio::test]
    async fn test_extract_keywords_to_news_keywords() {
        /*        let args = "http://127.0.0.1:8001/api/analysis/tfidf";
        let result = extract_news_keywords(args.to_string())
            .await
            .unwrap();
        println!("{:?}", result.data)*/

        /*        let args = "http://127.0.0.1:8001/api/analysis/extract_event";
        let result = extract_news_event(args.to_string())
            .await
            .unwrap();
        println!("{:?}", result.data);

        let args = "http://127.0.0.1:8001/api/analysis/merge_event?days=2";
        let result = extract_news_event(args.to_string())
            .await
            .unwrap();
        println!("{:?}", result.data)*/
    }
}
