use base64::{Engine as _, engine::general_purpose};
use common::api::ApiResponse;
use common::api::ItemResult;
use common::api::{AniItem, TaskItem};
use common::utils::date_utils::{get_today_slash, get_today_weekday};
use reqwest::Url;
use scraper::{Html, Selector};
use std::collections::HashMap;
use tracing::{debug, info};

pub async fn fetch_mikanani_image(url: String) -> Result<String, String> {
    // 新建异步 Reqwest 客户端
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Referer", "https://mikanani.me/")
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

pub async fn fetch_mikanani_ani_data(url: String) -> Result<ApiResponse<ItemResult>, String> {
    // 1. 发请求拿响应
    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Referer", "https://mikanani.me/")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // 2. 解析成 HTML 文本
    let body = response.text().await.map_err(|e| e.to_string())?;
    debug!(
        "解析从 Mikanani 获取到的 HTML，前 200 字符：\n{}",
        &body[..200.min(body.len())]
    );
    info!("成功获取蜜柑计划追番表数据");
    // 解析 HTML
    let document = Html::parse_document(&body);
    // 找到所有 <li> 节点
    let li_sel = Selector::parse("li").unwrap();
    // base_url 用于拼接相对链接
    let base_url = Url::parse(&url).map_err(|e| e.to_string())?;

    // 3. 初始化一个空的 result
    let mut result: ItemResult = HashMap::new();
    let weekday_str = get_today_weekday().name_cn.to_string();
    // 今天的日期，比如 "2025/07/13"
    let today_date = get_today_slash();
    // 动漫aniitem的列表
    let mut comics: Vec<TaskItem> = Vec::new();
    // 过滤出符合条件的 <li>
    for li in document.select(&li_sel) {
        // 必须有 <div class="num-node text-center">
        if li
            .select(&Selector::parse("div.num-node.text-center").unwrap())
            .next()
            .is_none()
        {
            continue;
        }
        // 且 <div class="date-text"> 包含 today_date
        if let Some(div) = li.select(&Selector::parse("div.date-text").unwrap()).next() {
            let text = div.text().collect::<String>();
            if !text.contains(&today_date) {
                continue;
            }
        } else {
            continue;
        }

        // 构建 Ani 并加入结果
        if let Some(item) = build_mikanani_item(&base_url, &li) {
            info!("识别到更新：{} {}", item.title, item.update_info);
            comics.push(TaskItem::Ani(item));
        }
    }
    info!("成功提取到 {} 部今日更新的动漫", comics.len());
    result.insert(weekday_str, comics);

    // 7. 返回包装后的结果
    Ok(ApiResponse::ok(result))
}

fn build_mikanani_item(base_url: &Url, li: &scraper::element_ref::ElementRef) -> Option<AniItem> {
    // <a class="an-text" title="..." href="...">
    let a_sel = Selector::parse("a.an-text").unwrap();
    let a = li.select(&a_sel).next()?;
    let title = a.value().attr("title").unwrap_or("").trim().to_string();

    // <div class="date-text">2025/07/13 20:00</div>
    let date_sel = Selector::parse("div.date-text").unwrap();
    let update_info = li
        .select(&date_sel)
        .next()
        .map(|d| d.text().collect::<String>().trim().to_string())
        .unwrap_or_default();

    // update_time 取空格前部分
    let update_time = update_info
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();

    // 图片 URL 在 <span class="js-expand_bangumi" data-src="...">
    let span_sel = Selector::parse("span.js-expand_bangumi").unwrap();
    let data_src = li
        .select(&span_sel)
        .next()
        .and_then(|s| s.value().attr("data-src"))?;
    let image_url = base_url.join(data_src).ok()?.to_string();

    // 详情 URL 来自 <a>.href
    let href = a.value().attr("href").unwrap_or("");
    let detail_url = base_url.join(href).ok()?.to_string();

    Some(AniItem {
        platform: "mikanani".to_string(),
        title,
        update_count: String::new(), // Python 里是空
        update_info,
        image_url,
        detail_url,
        update_time,
    })
}
