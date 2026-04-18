use crate::common::AppState;
use actix_web::{get, web, HttpResponse};
use futures_util::stream;
use serde::Serialize;
use sqlx::PgPool;
use std::time::Duration;
use tokio::sync::mpsc;

/// SSE 推送给前端的新闻条目
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NewsSseItem {
    id: i64,
    title: String,
    url: String,
    /// 新闻分类（复用 source 字段）
    category: String,
    /// 新闻来源平台
    news_from: String,
    /// 发布日期（YYYY-MM-DD）
    news_date: String,
}

/// 数据库查询用中间结构
#[derive(Debug, sqlx::FromRow)]
struct NewsRow {
    id: i64,
    title: String,
    url: String,
    source: Option<String>,
    published_at: chrono::NaiveDate,
}

/// 查询最新 N 条新闻（按 id 倒序）
async fn fetch_initial_news(pool: &PgPool, limit: i64) -> Vec<NewsRow> {
    sqlx::query_as::<_, NewsRow>(
        r#"
        SELECT id, title, url, source, published_at
        FROM news_item
        ORDER BY id DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// 查询 id 大于指定值的新增新闻（增量轮询）
async fn fetch_new_news(pool: &PgPool, after_id: i64, limit: i64) -> Vec<NewsRow> {
    sqlx::query_as::<_, NewsRow>(
        r#"
        SELECT id, title, url, source, published_at
        FROM news_item
        WHERE id > $1
        ORDER BY id ASC
        LIMIT $2
        "#,
    )
    .bind(after_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// 将数据库行序列化为 SSE 消息字节
fn row_to_sse_bytes(row: &NewsRow) -> Option<web::Bytes> {
    let source = row.source.clone().unwrap_or_else(|| "unknown".to_string());
    let item = NewsSseItem {
        id: row.id,
        title: row.title.clone(),
        url: row.url.clone(),
        category: source.clone(),
        news_from: source,
        news_date: row.published_at.to_string(),
    };
    let json = serde_json::to_string(&item).ok()?;
    Some(web::Bytes::from(format!("event: news\ndata: {json}\n\n")))
}

/// GET /api/news/stream
/// SSE 实时推送最新新闻（无需登录，公开接口）
///
/// 行为：
/// 1. 连接时立即推送最新 20 条历史新闻（从旧到新）
/// 2. 之后每 30 秒轮询增量数据并推送
/// 3. 每次轮询前发送 SSE 心跳注释，防止代理断连
#[get("/news/stream")]
pub async fn news_stream_sse(app_state: web::Data<AppState>) -> HttpResponse {
    let (tx, rx) = mpsc::channel::<Result<web::Bytes, actix_web::Error>>(64);
    let pool = app_state.db_pool.clone();

    tokio::spawn(async move {
        // 1. 初始推送：取最新 20 条，翻转为从旧到新推送
        let mut initial = fetch_initial_news(&pool, 20).await;
        initial.reverse();

        let mut last_id: i64 = initial.iter().map(|r| r.id).max().unwrap_or(0);

        for row in &initial {
            if let Some(bytes) = row_to_sse_bytes(row) {
                if tx.send(Ok(bytes)).await.is_err() {
                    return; // 客户端已断连
                }
            }
        }

        // 2. 周期性轮询新数据
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        interval.tick().await; // 跳过立即触发的第一次 tick

        loop {
            interval.tick().await;

            // 心跳注释，保持长连接不被代理断开
            if tx
                .send(Ok(web::Bytes::from(": ping\n\n")))
                .await
                .is_err()
            {
                return;
            }

            let new_rows = fetch_new_news(&pool, last_id, 50).await;
            for row in &new_rows {
                last_id = last_id.max(row.id);
                if let Some(bytes) = row_to_sse_bytes(row) {
                    if tx.send(Ok(bytes)).await.is_err() {
                        return;
                    }
                }
            }
        }
    });

    // 将 mpsc::Receiver 转为 Stream
    let sse_stream = stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    });

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(sse_stream)
}
