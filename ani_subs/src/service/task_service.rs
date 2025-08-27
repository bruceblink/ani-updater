use crate::dao::postgresql::ani_info_table::upsert_ani_info;
use ani_spiders::{AniItemResult, ApiResponse};
use common::utils::date_utils::get_today_weekday;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;

pub async fn run_task_service(
    ani_item_result: AniItemResult,
    pool: Arc<PgPool>,
) -> anyhow::Result<ApiResponse, String> {
    // 启动定时任务服务
    let weekday = get_today_weekday().name_cn.to_string();

    let items = match ani_item_result.get(&weekday) {
        Some(v) if !v.is_empty() => v,
        Some(_) => return Ok(ApiResponse::ok(json!({ "message": "没有可插入的数据" }))),
        None => return Ok(ApiResponse::err("获取今日动漫数据失败")),
    };

    for item in items {
        if let Err(e) = upsert_ani_info(item, &pool).await {
            return Ok(ApiResponse::err(format!("插入失败：{}", e)));
        }
    }
    Ok(ApiResponse::ok(json!({ "message": "save success" })))
}
