use anyhow::{Context, Result};
use common::api::ApiResponse;
use common::po::{HealthItem, ItemResult, TaskItem};
use common::utils::date_utils::get_today_weekday;
use std::collections::{HashMap, HashSet};
use tokio::task::JoinSet;
use tracing::{error, warn};

use crate::process_news_info::HTTP_CLIENT;

/// 获取健康检测的数据</br>
/// args: 待检测的url，如果是多个url,使用英文","隔开
pub async fn health_check(args: String) -> Result<ApiResponse<ItemResult>, String> {
    let sources: Vec<&str> = args.split(',').map(|s| s.trim()).collect();
    let mut result: ItemResult = HashMap::new();
    let weekday = get_today_weekday().name_cn.to_string();

    let mut join_set = JoinSet::new();

    // 添加所有任务到 JoinSet
    for arg in sources {
        let arg = arg.to_string();
        join_set.spawn(async move { health_check_single(&arg).await });
    }

    // 收集所有结果
    let mut all_items: HashSet<TaskItem> = HashSet::new();
    while let Some(task_result) = join_set.join_next().await {
        match task_result {
            Ok(Ok(item)) => {
                all_items.insert(TaskItem::Health(item));
            }
            Ok(Err(e)) => warn!("获取检测的url失败 {}", e),
            Err(e) => error!("任务执行失败 {}", e),
        }
    }
    result.insert(weekday, all_items);
    Ok(ApiResponse::ok(result))
}

async fn health_check_single(url: &str) -> Result<HealthItem> {
    let response = HTTP_CLIENT
        .get(url)
        .header("Referer", url)
        .send()
        .await
        .with_context(|| format!("请求待检测url {url} 失败"))?;

    // 检查HTTP状态码
    if !response.status().is_success() {
        anyhow::bail!("请求 {url} 错误- 状态码: {}", response.status());
    }
    // 将响应解析成json
    let json_value: serde_json::Value = response
        .json()
        .await
        .with_context(|| format!("解析 {url} 的JSON响应失败"))?;

    Ok(HealthItem {
        url: url.to_string(),
        result: json_value,
    })
}
