use crate::dao::list_all_news_info_by_page;
use crate::routes::NewsFilter;
use actix_web::web;
use common::api::{ApiResponse, NewsInfo2Item};
use common::po::{ItemResult, QueryPage, TaskItem};
use common::utils::date_utils::{DateFormat, format_now, get_today_weekday};
use sqlx::PgPool;
use std::collections::HashMap;

pub async fn extract_transform_news_info_to_item(
    db_pool: &PgPool,
) -> anyhow::Result<ApiResponse<ItemResult>, String> {
    let mut item_result: ItemResult = HashMap::new();
    let weekday = get_today_weekday().name_cn.to_string();
    let query = create_empty_query();
    let result = list_all_news_info_by_page(query, db_pool)
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
                    TaskItem::Extract(NewsInfo2Item {
                        id: news_info.id.to_string(),
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
    let now_str = format_now(DateFormat::Iso);
    let filter = NewsFilter {
        news_from: None,
        news_date: Option::from(now_str),
        extracted: Option::from(false),
    };

    let query_page = QueryPage {
        page: None,
        filter: Some(filter),
        page_size: None,
    };

    web::Query(query_page)
}
