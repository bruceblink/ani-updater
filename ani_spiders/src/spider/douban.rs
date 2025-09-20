use base64::{Engine as _, engine::general_purpose};
use common::api::{ApiResponse, VideoItem};
use serde_json::Value;
use tracing::{error, info};

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
pub async fn fetch_douban_movie_data(url: String) -> Result<ApiResponse<Vec<VideoItem>>, String> {
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Referer", "https://movie.douban.com/")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json_value: Value = response.json().await.map_err(|e| e.to_string())?;

    let result = process_json_value(&json_value);
    Ok(ApiResponse::ok(result))
}

fn process_json_value(json_value: &Value) -> Vec<VideoItem> {
    // 获取响应报文中名称为 items的 JSON数组
    let items = match json_value["items"].as_array() {
        Some(arr) => arr,
        None => {
            error!("items 字段不是数组: {}", json_value);
            return Vec::new();
        }
    };
    info!("成功获取豆瓣今日热门电影数据");

    let mut videos: Vec<VideoItem> = Vec::new();

    for item in items {
        let item = parse_item_to_video_item(item);
        info!("识别到更新：{} {:?}", item.title, item.card_subtitle);
        videos.push(item);
    }

    info!("成功提取到 {} 部今日更新的动漫", videos.len());

    videos
}

fn parse_item_to_video_item(item: &Value) -> VideoItem {
    VideoItem {
        id: item.get("id").unwrap().to_string(),
        title: item.get("title").unwrap().to_string(),
        rating: item.get("rating").cloned(),
        pic: item.get("pic").cloned(),
        is_new: item.get("is_new").unwrap().as_bool(),
        uri: item.get("pic").unwrap().to_string(),
        episodes_info: Option::from(item.get("episodes_info").unwrap().to_string()),
        card_subtitle: item.get("card_subtitle").unwrap().to_string(),
        r#type: item.get("pic").unwrap().to_string(),
    }
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
