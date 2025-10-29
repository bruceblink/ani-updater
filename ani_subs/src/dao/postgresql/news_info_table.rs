use crate::routes::NewsInfo;
use anyhow::Result;
use common::api::NewsItem;
use sqlx::PgPool;

/// 新闻信息插入新记录
pub async fn upsert_news_info(item: &NewsItem, db_pool: &PgPool) -> Result<()> {
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
    .bind(&item.news_from)
    .bind(&item.data)
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("插入或更新 news_info {:?} 失败: {}", item, e);
        anyhow::anyhow!(e)
    })?;

    Ok(())
}
