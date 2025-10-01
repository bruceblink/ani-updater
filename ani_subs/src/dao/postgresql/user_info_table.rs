use crate::domain::dto::{NewUser, UserDto, UserIdentityDto};
use crate::domain::po::UserInfo;
use chrono_tz::Asia::Shanghai;
use sqlx::{PgPool, Row};

/// 根据用户名查询用户信息
pub async fn get_user_by_username(
    username: String,
    db_pool: &PgPool,
) -> anyhow::Result<Option<UserDto>> {
    let rec = sqlx::query_as::<_, UserInfo>(
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
                FROM user_info
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

/// 新增用户
pub async fn insert_users(users: &[NewUser], pool: &PgPool) -> anyhow::Result<()> {
    if users.is_empty() {
        return Ok(());
    }
    // 取第一个 user 的字段数量（这里假设所有 User 结构字段数相同）
    // 注意：这里我们只取要插入的字段，不要像 id、created_at 这种默认生成的
    let field_count = 5; // email, username, password, display_name, avatar_url

    // 1️⃣ 动态拼接 VALUES 部分
    let mut placeholders = Vec::new();
    for i in 0..users.len() {
        let base = i * field_count;
        let group: Vec<String> = (1..=field_count)
            .map(|j| format!("${}", base + j))
            .collect();
        placeholders.push(format!("({})", group.join(", ")));
    }

    // 2️⃣ 拼接完整 SQL
    let query = format!(
        "INSERT INTO user_info (email, username, password, display_name, avatar_url) VALUES {}",
        placeholders.join(", ")
    );

    // 3️⃣ 绑定参数
    let mut sql = sqlx::query(&query);
    for user in users {
        sql = sql
            .bind(&user.email)
            .bind(&user.username)
            .bind(&user.password)
            .bind(&user.display_name)
            .bind(&user.avatar_url);
    }

    // 4️⃣ 执行
    sql.execute(pool).await?;
    Ok(())
}

/// 新增第三方登录用户 <br>
/// 返回 <br>
/// (user_id, refresh_token)
pub async fn upsert_user_with_third_part(
    user: &UserIdentityDto,
    pool: &PgPool,
) -> anyhow::Result<(i64, String)> {
    let row = sqlx::query(
        r#"
            WITH upsert_user AS (
                INSERT INTO user_info (email, username, display_name, avatar_url)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (email) DO UPDATE
                    SET username = EXCLUDED.username,
                        display_name = EXCLUDED.display_name,
                        avatar_url = EXCLUDED.avatar_url
                RETURNING id
            ),
            ins_identity AS (
                INSERT INTO user_identities (user_id, provider, provider_uid, access_token, token_expires_at)
                SELECT id, $5, $6, $7, now() + interval '30 days'
                FROM upsert_user
                ON CONFLICT (provider, provider_uid) DO NOTHING
                RETURNING user_id
            )
            INSERT INTO refresh_tokens (user_id, token, expires_at)
            SELECT id, $8, $9
            FROM upsert_user
            RETURNING user_id, token;
            "#
        )
        .bind(&user.email)
        .bind(&user.username)
        .bind(&user.display_name)
        .bind(&user.avatar_url)
        .bind(&user.provider)
        .bind(&user.provider_user_id)
        .bind(user.access_token.as_deref())
        .bind(&user.refresh_token)
        .bind(user.expires_at)
        .fetch_one(pool)
        .await
        .map_err(|e| {
            tracing::error!("插入或更新用户 {:?} 失败: {}", user, e);
            anyhow::anyhow!(e)
        })?;

    let user_id: i64 = row.get("user_id");
    let refresh_token: String = row.get("token");
    Ok((user_id, refresh_token))
}
