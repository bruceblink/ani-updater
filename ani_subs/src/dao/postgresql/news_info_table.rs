use anyhow::Result;
use common::api::NewsItem;
use serde_json::json;
use sqlx::PgPool;

/// 新闻信息插入新记录
pub async fn upsert_news_info(news_item: &NewsItem, db_pool: &PgPool) -> Result<()> {
    let data = json!({
        "items": news_item.items
    });
    let _ = sqlx::query(
        r#"
        INSERT INTO public.news_info (
            news_from,
            data
        ) VALUES ($1, $2)
        ON CONFLICT (news_from, news_date) DO UPDATE SET
            news_from = EXCLUDED.news_from,
            data = EXCLUDED.data
        "#,
    )
    .bind(&news_item.id)
    .bind(data)
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("插入或更新 news_info {:?} 失败: {}", news_item, e);
        anyhow::anyhow!(e)
    })?;

    Ok(())
}
