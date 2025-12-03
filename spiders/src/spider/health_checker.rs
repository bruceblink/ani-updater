use anyhow::{Context, Result};
use common::api::{ApiResponse, ItemResult, TaskItem};
use common::utils::date_utils::get_today_weekday;
use std::collections::HashMap;
use tokio::task::JoinSet;
use tracing::{error, warn};

/// 获取健康检测的数据</br>
/// args: 待检测的url，如果是多个url,使用英文","隔开
pub async fn health_check(args: String) -> Result<ApiResponse<ItemResult>, String> {
    let sources: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
    let mut result: ItemResult = HashMap::new();
    let client = reqwest::Client::new();
    let weekday = get_today_weekday().name_cn.to_string();

    let mut join_set = JoinSet::new();

    // 添加所有任务到 JoinSet
    for arg in sources {
        let client = client.clone();
        let arg = arg.to_string();
        join_set.spawn(async move { health_check_single(&client, &arg).await });
    }

    // 收集所有结果
    let mut all_items: Vec<TaskItem> = Vec::new();
    while let Some(task_result) = join_set.join_next().await {
        match task_result {
            Ok(Ok(item)) => all_items.push(TaskItem::Health(item)),
            Ok(Err(e)) => warn!("获取检测的url失败: {}", e),
            Err(e) => error!("任务执行失败: {}", e),
        }
    }
    result.insert(weekday, all_items);
    Ok(ApiResponse::ok(result))
}

async fn health_check_single(client: &reqwest::Client, arg: &str) -> Result<serde_json::Value> {
    let response = client
        .get(arg)
        .header("Referer", arg)
        .send()
        .await
        .with_context(|| format!("请求待检测url失败: {}", arg))?;

    // 检查HTTP状态码
    if !response.status().is_success() {
        anyhow::bail!("HTTP错误: {} - 状态码: {}", arg, response.status());
    }
    // 将响应解析成json
    let json_value: serde_json::Value = response
        .json()
        .await
        .with_context(|| format!("解析JSON响应失败: {}", arg))?;

    Ok(json_value)
}

#[cfg(test)]
mod test {
    use crate::spider::health_checker::health_check;

    #[tokio::test]
    async fn test_fetch_douban_image() {
        let args = "http://127.0.0.1:8000/health, http://127.0.0.1:8000/health";
        let result = health_check(args.to_string()).await.unwrap();
        println!("{:?}", result.data)
    }
}
