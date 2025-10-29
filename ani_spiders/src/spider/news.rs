use common::api::{ApiResponse, ItemResult, NewsItem, TaskItem};
use common::utils::date_utils::get_today_weekday;
use serde_json::{Value, from_value};
use std::collections::HashMap;
use tokio::task::JoinSet;
use tracing::error;

/// 获取最新新闻的数据
pub async fn fetch_latest_news_data(args: String) -> Result<ApiResponse<ItemResult>, String> {
    let sources: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
    let mut result: ItemResult = HashMap::new();
    let client = reqwest::Client::new();
    let weekday = get_today_weekday().name_cn.to_string();

    let mut join_set = JoinSet::new();

    // 添加所有任务到 JoinSet
    for arg in sources {
        let client = client.clone();
        let arg = arg.to_string();
        join_set.spawn(async move { fetch_single_news_source(&client, &arg).await });
    }

    // 收集所有结果
    let mut all_news: Vec<TaskItem> = Vec::new();
    while let Some(task_result) = join_set.join_next().await {
        match task_result {
            Ok(Ok(news_item)) => all_news.push(TaskItem::News(news_item)),
            Ok(Err(e)) => error!("获取新闻源失败: {}", e),
            Err(e) => error!("任务执行失败: {}", e),
        }
    }
    result.insert(weekday, all_news);
    Ok(ApiResponse::ok(result))
}

async fn fetch_single_news_source(
    client: &reqwest::Client,
    arg: &str,
) -> anyhow::Result<NewsItem, String> {
    let url = format!("https://news.likanug.top/api/s?id={}", arg);

    let response = client
        .get(&url)
        .header("Referer", "https://news.likanug.top/")
        .send()
        .await
        .map_err(|e| format!("请求失败: {} - {}", arg, e))?;

    let json_value: Value = response
        .json()
        .await
        .map_err(|e| format!("解析JSON失败: {} - {}", arg, e))?;
    let data: NewsItem =
        from_value(json_value).map_err(|e| format!("解析JSON失败: {} - {}", arg, e))?;

    Ok(data)
}

#[cfg(test)]
mod test {
    use crate::news::fetch_latest_news_data;

    #[tokio::test]
    async fn test_fetch_douban_image() {
        let args = "baidu,bilibili-hot-search,bilibili-hot-video,bilibili-ranking";
        let result = fetch_latest_news_data(args.to_string()).await.unwrap();
        println!("{:?}", result.data)
    }
}
