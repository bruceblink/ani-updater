use base64::{Engine as _, engine::general_purpose};
use common::api::ApiResponse;
use common::po::{ItemResult, TaskItem, VideoItem};
use common::utils::date_utils::get_today_weekday;
use serde_json::{Value, from_value};
use std::collections::HashMap;
use tracing::{error, info, warn};

pub async fn fetch_douban_image(url: String) -> Result<String, String> {
    // 新建异步 Reqwest 客户端
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Referer", "https://movie.douban.com/")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // 先把 Content-Type 拷贝到一个拥有 String
    let ct: String = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "image/jpeg".to_string());

    // 这时 resp 不再被借用，可以放心移动
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;

    // 转 base64，并拼成 Data URL
    let b64 = general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{ct};base64,{b64}"))
}

/// 获取豆瓣热门电影的数据
pub async fn fetch_douban_movie_data(url: String) -> Result<ApiResponse<ItemResult>, String> {
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Referer", "https://movie.douban.com/")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json_value: Value = response.json().await.map_err(|e| e.to_string())?;

    let result: ItemResult = process_json_value(&json_value);
    Ok(ApiResponse::ok(result))
}

fn process_json_value(json_value: &Value) -> ItemResult {
    let weekday = get_today_weekday().name_cn.to_string();
    // 获取响应报文中名称为 items的 JSON数组
    let items = match json_value["items"].as_array() {
        Some(arr) => arr,
        None => {
            error!("items 字段不是数组: {}", json_value);
            return HashMap::new();
        }
    };
    info!("成功获取电影数据");

    let mut videos: Vec<TaskItem> = Vec::new();

    for item in items {
        match from_value::<VideoItem>(item.clone()) {
            Ok(it) => {
                videos.push(TaskItem::Video(it.clone()));
                info!("识别到更新：{} {:?}", it.title, it.card_subtitle);
            }
            Err(e) => warn!("解析失败: {e}"),
        };
    }
    info!("成功提取到 {} 部电影", videos.len());
    let mut result = HashMap::new();
    result.insert(weekday, videos);

    result
}

#[cfg(test)]
mod test {
    use crate::douban::fetch_douban_movie_data;

    const URL: &str = "https://m.douban.com/rexxar/api/v2/subject/recent_hot/movie";

    #[tokio::test]
    async fn test_fetch_douban_image() {
        let result = fetch_douban_movie_data(URL.parse().unwrap()).await.unwrap();
        println!("{:?}", result.data)
    }
}
