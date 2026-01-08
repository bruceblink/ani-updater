use chrono::Utc;
use chrono_tz::Asia::Shanghai;
use common::dto::{NewUser, UserDto, UserIdentityDto};
use common::po::UserInfo;
use serde::Deserialize;
use sqlx::FromRow;
use sqlx::PgPool;

/// 根据email名查询用户信息
pub async fn get_user_by_email(email: String, db_pool: &PgPool) -> anyhow::Result<Option<UserDto>> {
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
                    updated_at,
                    tenant_id,
                    org_id,
                    plan,
                    token_version,
                    status,
                    locked_until,
                    failed_login_attempts
                FROM user_info
                WHERE
                  email = $1
            ;"#,
    )
    .bind(email)
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
        tenant_id: user.tenant_id,
        org_id: user.org_id,
        plan: user.plan,
        token_version: user.token_version,
        status: user.status,
        locked_until: user.locked_until,
        failed_login_attempts: user.failed_login_attempts,
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

#[derive(FromRow, Debug, Deserialize, Clone)]
pub struct UserInfoWithTokenDTO {
    pub user_id: i64,
    pub token: String,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub email: String,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub tenant_id: String,
    pub org_id: Option<String>,
    pub plan: String,
    pub token_version: i64,
    pub status: String,
    pub locked_until: Option<chrono::DateTime<Utc>>,
    pub failed_login_attempts: i32,
    pub roles: Vec<String>,
}

/// 新增第三方登录用户 <br>
/// 返回 <br>
/// (user_id, refresh_token)
pub async fn upsert_user_with_third_part(
    user: &UserIdentityDto,
    pool: &PgPool,
) -> anyhow::Result<UserInfoWithTokenDTO> {
    let row: UserInfoWithTokenDTO = sqlx::query_as(
        r#"
                WITH upsert_user AS (
                    INSERT INTO user_info (email, username, display_name, avatar_url)
                    VALUES ($1, $2, $3, $4)
                    ON CONFLICT (email) DO UPDATE
                        SET username = EXCLUDED.username,
                            display_name = EXCLUDED.display_name,
                            avatar_url = EXCLUDED.avatar_url
                    RETURNING id, email, username, display_name, avatar_url, created_at, updated_at, tenant_id, org_id, plan, token_version, status, locked_until, failed_login_attempts
                ),
                insert_identity AS (
                    INSERT INTO user_identities (user_id, provider, provider_uid, access_token, token_expires_at)
                    SELECT id, $5, $6, $7, $9
                    FROM upsert_user
                    ON CONFLICT (provider, provider_uid) DO NOTHING
                    RETURNING user_id
                ),
                insert_refresh_token AS (
                    INSERT INTO refresh_tokens (user_id, token, expires_at)
                    SELECT id, $8, $9
                    FROM upsert_user
                    RETURNING user_id, token, expires_at
                ),
                insert_roles AS (
                    INSERT INTO user_roles (user_id, role_id)
                    SELECT u.id, r.id
                    FROM upsert_user u
                    JOIN roles r ON r.name = 'user'  -- 默认角色设为 'user'
                    ON CONFLICT (user_id, role_id) DO NOTHING
                    RETURNING user_id
                ),
                user_info_with_roles AS (
                        SELECT u.id AS user_id,
                               u.email,
                               u.username,
                               u.display_name,
                               u.avatar_url,
                               u.created_at,
                               u.updated_at,
                               u.tenant_id,
                               u.org_id,
                               u.plan,
                               u.token_version,
                               u.status,
                               u.locked_until,
                               u.failed_login_attempts,
                               r.token,
                               r.expires_at,
                               array_agg(role.name) AS roles
                        FROM upsert_user u
                        LEFT JOIN insert_refresh_token r ON u.id = r.user_id
                        LEFT JOIN user_roles ur ON ur.user_id = u.id
                        LEFT JOIN roles role ON role.id = ur.role_id
                        GROUP BY u.id, u.email, u.username, u.display_name, u.avatar_url, u.created_at, u.updated_at, u.tenant_id, u.org_id, u.plan, u.token_version, u.status, u.locked_until, u.failed_login_attempts, r.token, r.expires_at
                    )
                    SELECT * FROM user_info_with_roles;
        "#
    )
        .bind(&user.email)         // 用户邮箱
        .bind(&user.username)      // 用户名
        .bind(&user.display_name)  // 显示名
        .bind(&user.avatar_url)    // 头像
        .bind(&user.provider)      // 登录提供商（如GitHub）
        .bind(&user.provider_user_id) // 第三方用户唯一ID
        .bind(user.access_token.as_deref()) // access_token
        .bind(&user.refresh_token) // refresh_token
        .bind(user.expires_at)     // refresh_token的过期时间
        .fetch_one(pool)
        .await
        .map_err(|e| {
            tracing::error!("插入或更新用户 {:?} 失败: {}", user, e);
            anyhow::anyhow!(e)
        })?;

    Ok(row)
}
