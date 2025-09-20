use base64::{Engine as _, engine::general_purpose};
use chrono::{Datelike, Local};
use common::api::ApiResponse;
use common::api::ItemResult;
use common::api::{AniItem, TaskItem};
use common::utils::date_utils::{get_today_slash, get_today_weekday};
use common::utils::{clean_text, extract_number};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{error, info};

pub async fn fetch_iqiyi_image(url: String) -> Result<String, String> {
    // 新建异步 Reqwest 客户端
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Referer", "https://www.iqiyi.com/")
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

pub async fn fetch_iqiyi_ani_data(url: String) -> Result<ApiResponse<ItemResult>, String> {
    // 1. 发请求拿 JSON
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Referer", "https://www.iqiyi.com/")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // 2. 反序列化成 serde_json::Value
    let json_value: Value = response.json().await.map_err(|e| e.to_string())?;

    // 3. 处理解析成 AniItemResult
    let result: ItemResult = process_json_value(&json_value);

    // 4. 返回统一包装
    Ok(ApiResponse::ok(result))
}

fn process_json_value(json_value: &Value) -> ItemResult {
    // 验证响应格式
    if json_value.get("code") != Some(&Value::from(0)) {
        error!("接口返回错误状态: {}", json_value);
        return HashMap::new();
    }

    let items = match json_value.get("items").and_then(|v| v.as_array()) {
        Some(arr) if !arr.is_empty() => arr,
        _ => {
            error!("缺少有效的items数组: {}", json_value);
            return HashMap::new();
        }
    };

    info!("成功获取爱奇艺追番表数据");

    // 提前计算周信息
    let current_weekday = Local::now().weekday().num_days_from_monday() as usize;
    let weekday_str = get_today_weekday().name_cn.to_string();

    // 查找追番表数据
    let today_data = items
        .iter()
        .find(|item| item.get("title") == Some(&Value::from("追番表")))
        .and_then(|item| item.get("video"))
        .and_then(|video| video.as_array())
        .and_then(|video_list| video_list.get(current_weekday))
        .and_then(|day_data| day_data.get("data"))
        .and_then(|data| data.as_array());

    // 处理追番数据
    let mut result = HashMap::new();

    match today_data {
        Some(list) if !list.is_empty() => {
            let items: Vec<TaskItem> = list
                .iter()
                .filter_map(parse_item)
                .inspect(|res| {
                    info!("识别到更新：{} {}", res.title, res.update_info);
                })
                .map(TaskItem::Ani) // AniItem 包一层 TaskItem::Ani
                .collect();
            info!("成功提取到 {} 部今日更新的动漫", items.len());
            result.insert(weekday_str, items);
        }
        Some(_) => {
            // 空数组
            info!("今日没有更新");
            result.insert(weekday_str, vec![]);
        }
        None => {
            error!("未找到今日追番数据，当前星期索引: {}", current_weekday);
        }
    }

    result
}

fn parse_item(ep: &Value) -> Option<AniItem> {
    let title = ep.get("display_name")?.as_str().unwrap_or("").to_string();
    let raw_update_info = ep
        .get("dq_updatestatus")?
        .as_str()
        .unwrap_or("")
        .trim()
        .to_string();
    let update_count = extract_number(&raw_update_info)?.to_string();

    let image_url = ep
        .get("image_cover")
        .or_else(|| ep.get("image_url_normal"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    //let image_url = image_url.replace("http", "https");

    let detail_url = ep
        .get("page_url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Some(AniItem {
        platform: "iqiyi".to_string(), // 平台名可以写死或传参
        title: clean_text(&title),
        update_count,
        update_info: raw_update_info,
        image_url,
        detail_url,
        update_time: get_today_slash(),
    })
}
