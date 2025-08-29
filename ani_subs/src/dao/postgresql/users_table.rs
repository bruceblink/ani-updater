use crate::domain::dto::UserDto;
use crate::domain::po::User;
use chrono_tz::Asia::Shanghai;
use sqlx::PgPool;

/// 根据 id 查询单条
pub async fn get_user_by_username(
    username: String,
    db_pool: &PgPool,
) -> anyhow::Result<Option<UserDto>> {
    let rec = sqlx::query_as::<_, User>(
        r#"
                SELECT 
                    id,
                    email,
                    username,
                    password,
                    display_name,
                    avatar_url,
                    created_at,
                    updated_at
                FROM users
                WHERE
                  username = $1
            ;"#,
    )
    .bind(username)
    .fetch_optional(db_pool)
    .await?;
    // 转换时间字段到上海时区
    let dto = rec.map(|user| UserDto {
        id: user.id,
        email: user.email,
        username: user.username,
        password: user.password,
        display_name: user.display_name,
        avatar_url: user.avatar_url,
        created_at: user.created_at.with_timezone(&Shanghai).to_rfc3339(),
        updated_at: user
            .updated_at
            .map(|dt| dt.with_timezone(&Shanghai).to_rfc3339()),
    });
    Ok(dto)
}
