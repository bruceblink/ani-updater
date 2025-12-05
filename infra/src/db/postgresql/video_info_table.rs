use anyhow::Result;
use common::po::VideoItem;
use sqlx::PgPool;

/// 视频信息插入新记录
pub async fn upsert_video_info(item: &VideoItem, db_pool: &PgPool) -> Result<()> {
    let _ = sqlx::query(
        r#"
        INSERT INTO public.video_info (
            title,
            rating,
            pic,
            is_new,
            uri,
            episodes_info,
            card_subtitle,
            type
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (title, episodes_info) DO UPDATE SET
            rating = EXCLUDED.rating,
            pic = EXCLUDED.pic,
            is_new = EXCLUDED.is_new,
            uri = EXCLUDED.uri,
            episodes_info = EXCLUDED.episodes_info,
            card_subtitle = EXCLUDED.card_subtitle
        "#,
    )
    .bind(&item.title)
    .bind(&item.rating)
    .bind(&item.pic)
    .bind(item.is_new)
    .bind(&item.id) // 入库时 用原API获取的豆瓣ID替换 URI字段
    .bind(&item.episodes_info)
    .bind(&item.card_subtitle)
    .bind(&item.r#type)
    .execute(db_pool)
    .await
    .map_err(|e| {
        tracing::error!("插入或更新 video_info {:?} 失败: {}", item, e);
        anyhow::anyhow!(e)
    })?;

    Ok(())
}
