use anyhow::{Context, Result};
use common::api::ApiResponse;
use common::po::{ItemResult, NewsInfo, TaskItem};
use common::utils::date_utils::get_today_weekday;
use serde_json::from_value;
use std::collections::{HashMap, HashSet};
use tokio::task::JoinSet;
use tracing::{error, warn};

/// 获取最新新闻的数据
pub async fn fetch_latest_news_data(
    api_urls: String,
    args: String,
) -> Result<ApiResponse<ItemResult>, String> {
    let urls: Vec<&str> = api_urls.split(';').map(|s| s.trim()).collect();
    let sources: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
    let mut result: ItemResult = HashMap::new();
    let client = reqwest::Client::new();
    let weekday = get_today_weekday().name_cn.to_string();

    let mut join_set = JoinSet::new();

    for url in urls.clone() {
        // 添加所有任务到 JoinSet
        for arg in sources.clone() {
            let client = client.clone();
            let url = url.to_string();
            let arg = arg.to_string();
            join_set.spawn(async move { fetch_single_news_source(&client, &url, &arg).await });
        }
    }

    // 收集所有结果
    let mut all_news: HashSet<TaskItem> = HashSet::new();
    while let Some(task_result) = join_set.join_next().await {
        match task_result {
            Ok(Ok(news_item)) => {
                all_news.insert(TaskItem::News(news_item));
            }
            Ok(Err(e)) => warn!("获取新闻源失败: {}", e),
            Err(e) => error!("任务执行失败: {}", e),
        }
    }
    result.insert(weekday, all_news);
    Ok(ApiResponse::ok(result))
}

async fn fetch_single_news_source(
    client: &reqwest::Client,
    url: &str,
    arg: &str,
) -> Result<NewsInfo> {
    let api_url = format!("{url}/api/s?id={arg}");

    let response = client
        .get(&api_url)
        .header("Referer", url)
        .send()
        .await
        .with_context(|| format!("请求新闻源: {} 失败", &api_url))?;

    // 检查HTTP状态码
    if !response.status().is_success() {
        anyhow::bail!("HTTP错误: {} - 状态码: {}", arg, response.status());
    }
    // 将响应解析成json
    let json_value: serde_json::Value = response
        .json()
        .await
        .with_context(|| format!("解析JSON响应失败: {}", arg))?;

    from_value(json_value).with_context(|| format!("反序列化NewsItem失败: {}", arg))
}

#[cfg(test)]
mod test {
    use crate::spider::news::fetch_latest_news_data;

    #[tokio::test]
    async fn test_fetch_latest_news() {
        let urls = "https://news.likanug.top;https://latest-news-pink.vercel.app";
        let args = "v2ex-share,36kr,kuaishou,bilibili-hot-video,bilibili-ranking,36kr-quick,bilibili,bilibili-hot-search,sputniknewscn";
        let result = fetch_latest_news_data(urls.to_string(), args.to_string())
            .await
            .unwrap();
        println!("{:?}", result.data)
    }

    #[tokio::test]
    async fn test_fetch_latest_news_with_empty_url() {
        let urls = "";
        let args = "v2ex-share,36kr,kuaishou,bilibili-hot-video,bilibili-ranking,36kr-quick,bilibili,bilibili-hot-search,sputniknewscn";
        let result = fetch_latest_news_data(urls.to_string(), args.to_string())
            .await
            .unwrap();
        println!("{:?}", result.data)
    }
}
