use actix_web::web;
use common::NewsFilter;
use common::api::{ApiResponse, NewsInfo2Item};
use common::po::{HealthItem, ItemResult, QueryPage, TaskItem};
use common::utils::date_utils::get_today_weekday;
use infra::{list_all_news_info_by_page, upsert_news_info_extracted_state, upsert_news_item};
use serde_json::json;
use sqlx::{PgPool, Pool};
use std::collections::HashMap;
use std::sync::Arc;

pub async fn query_news_info_to_extract(
    db_pool: Arc<PgPool>,
) -> anyhow::Result<ApiResponse<ItemResult>, String> {
    let mut item_result: ItemResult = HashMap::new();
    let weekday = get_today_weekday().name_cn.to_string();
    let query = create_empty_query();
    let result = list_all_news_info_by_page(query, &db_pool)
        .await
        .map_err(|e| e.to_string())?;

    let res: Vec<TaskItem> = result
        .items
        .iter()
        .flat_map(|news_info| {
            // 内层data字段的items 数组
            news_info.data["items"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|item| {
                    TaskItem::ExtractNewsItem(NewsInfo2Item {
                        id: news_info.id,
                        news_from: news_info.news_from.clone(),
                        name: news_info.name.clone(),
                        news_date: news_info.news_date, // NaiveDate 类型，不用 clone

                        // ====== 来自内层 item 的字段 ======
                        news_item_id: item["id"].as_str().unwrap_or("").to_string(),
                        title: item["title"].as_str().unwrap_or("").to_string(),
                        url: item["url"].as_str().unwrap_or("").to_string(),
                        content: item["hover"].as_str().map(|s| s.to_string()),

                        // 额外字段直接全保存
                        extra: item.clone(),
                    })
                })
                .collect::<Vec<TaskItem>>()
        })
        .collect();
    item_result.insert(weekday, res);

    Ok(ApiResponse::ok(item_result))
}

/// 创建空的news_info分页查询条件
fn create_empty_query() -> web::Query<QueryPage<NewsFilter>> {
    //let now_str = format_now(DateFormat::Iso);
    let filter = NewsFilter {
        news_from: None,
        news_date: None,
        extracted: Option::from(false),
    };

    let query_page = QueryPage {
        page: None,
        filter: Some(filter),
        page_size: Some(50),
    };

    web::Query(query_page)
}

pub async fn extract_news_item(
    new_item: &NewsInfo2Item,
    pool: &Pool<sqlx::postgres::Postgres>,
) -> Result<(), sqlx::Error> {
    // 开启事务
    let mut tx = pool.begin().await?;

    // 把事务引用传给两个 upsert 函数
    upsert_news_item(new_item, &mut tx).await?;
    upsert_news_info_extracted_state(new_item, &mut tx).await?;

    // 事务提交
    tx.commit().await?;
    Ok(())
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

#[cfg(test)]
mod test {

    #[tokio::test]
    async fn test_extract_keywords_to_news_keywords() {
        /*        let args = "http://127.0.0.1:8001/api/analysis/tfidf";
        let result = extract_news_keywords(args.to_string())
            .await
            .unwrap();
        println!("{:?}", result.data)*/
    }
}
