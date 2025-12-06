use actix_web::web;
use common::NewsFilter;
use common::api::{ApiResponse, NewsInfo2Item};
use common::po::{ItemResult, QueryPage, TaskItem};
use common::utils::date_utils::{DateFormat, format_now, get_today_weekday};
use infra::{list_all_news_info_by_page, upsert_news_info_extracted_state, upsert_news_item};
use sqlx::{PgPool, Pool};
use std::collections::HashMap;
use std::sync::Arc;

pub async fn extract_transform_news_info_to_item(
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
                    TaskItem::Extract(NewsInfo2Item {
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

pub async fn process_news(
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
