use crate::dao::postgresql::run_query;
use crate::domain::dto::AniInfoDto;
use crate::domain::po::AniInfo;
use ani_spiders::AniItem;
use anyhow::Result;
use chrono_tz::Asia::Shanghai;
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
    .map_err(|e| {
        tracing::error!("插入或更新 ani_info {:?} 失败: {}", item, e);
        anyhow::anyhow!(e)
    })?;

    Ok(())
}

/// 根据 id 查询单条
pub async fn get_ani_info_by_id(id: i64, db_pool: &PgPool) -> Result<Option<AniInfoDto>> {
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
    // 转换时间字段到上海时区
    let dto = rec.map(|ani| AniInfoDto {
        id: ani.id,
        title: ani.title,
        update_count: ani.update_count,
        update_info: ani.update_info,
        image_url: ani.image_url,
        detail_url: ani.detail_url,
        update_time: ani.update_time.with_timezone(&Shanghai).to_rfc3339(),
        platform: ani.platform,
    });
    Ok(dto)
}

/// 查询所有动漫信息
pub async fn list_all_ani_info(_title: String, db_pool: &PgPool) -> Result<Vec<AniInfoDto>> {
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
                    update_time >= current_date
                ORDER BY update_time DESC
            ;"#,
    );
    // 调用通用的 run_query
    let list = run_query(db_pool, query).await?;
    let list = list
        .into_iter()
        .map(|ani| AniInfoDto {
            id: ani.id,
            title: ani.title,
            update_count: ani.update_count,
            update_info: ani.update_info,
            image_url: ani.image_url,
            detail_url: ani.detail_url,
            update_time: ani.update_time.with_timezone(&Shanghai).to_rfc3339(),
            platform: ani.platform,
        })
        .collect();
    Ok(list)
}
