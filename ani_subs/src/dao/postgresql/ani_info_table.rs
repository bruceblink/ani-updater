use crate::dao::postgresql::run_query;
use crate::domain::po::AniInfo;
use ani_spiders::AniItem;
use anyhow::Result;
use sqlx::PgPool;

/// 动漫信息插入新记录
pub async fn upsert_ani_info(item: &AniItem, db_pool: &PgPool) -> Result<()> {
    let _ = sqlx::query(
        r#"
        INSERT INTO ani_info (
            title,
            update_count,
            update_info,
            image_url,
            detail_url,
            platform
        ) VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (title, platform, update_count) DO UPDATE SET
            update_info = EXCLUDED.update_info,
            image_url = EXCLUDED.image_url,
            detail_url = EXCLUDED.detail_url
        "#,
    )
    .bind(&item.title)
    .bind(&item.update_count)
    .bind(&item.update_info)
    .bind(&item.image_url)
    .bind(&item.detail_url)
    .bind(&item.platform)
    .execute(db_pool)
    .await
    .map_err(|e| anyhow::anyhow!("插入或更新 ani_info {:?} 失败: {}", item, e))?;

    Ok(())
}

/// 根据 id 查询单条
pub async fn get_ani_info_by_id(id: i64, db_pool: &PgPool) -> Result<Option<AniInfo>> {
    let rec = sqlx::query_as::<_, AniInfo>(
        r#"
                SELECT id,
                    title,
                    update_count,
                    update_info,
                    image_url,
                    detail_url,
                    update_time,
                    platform
                FROM ani_info
                WHERE
                  id = $1
            ;"#,
    )
    .bind(id)
    .fetch_optional(db_pool)
    .await?;
    Ok(rec)
}

/// 查询所有动漫信息
pub async fn list_all_ani_info(title: String, db_pool: &PgPool) -> Result<Vec<AniInfo>> {
    // 构造带绑定参数的 QueryAs
    let query = sqlx::query_as::<_, AniInfo>(
        r#"
                SELECT id,
                    title,
                    update_count,
                    update_info,
                    image_url,
                    detail_url,
                    update_time,
                    platform
                FROM ani_info
                WHERE
                  title LIKE '%' || $1 || '%'
                ORDER BY update_time DESC
            ;"#,
    )
    .bind(title);
    // 调用通用的 run_query
    let list = run_query(db_pool, query).await?;
    Ok(list)
}
