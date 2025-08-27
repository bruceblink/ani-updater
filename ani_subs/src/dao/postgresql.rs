pub mod ani_info_table;

use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::query::QueryAs;
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

/// 通用动态查询函数，T 是查询结果类型
pub async fn query_with_condition<T, S>(
    pool: &PgPool,
    table: &str,
    conditions: &S,
) -> Result<Vec<T>>
where
    T: for<'r> FromRow<'r, PgRow> + Send + Unpin,
    S: Serialize,
{
    let cond_json = serde_json::to_value(conditions)?;

    let mut builder = QueryBuilder::<Postgres>::new(format!("SELECT * FROM {}", table));

    let mut first = true;
    if let Value::Object(map) = cond_json {
        for (field, value) in map {
            // 忽略 Null 值
            if value.is_null() {
                continue;
            }

            if !first {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
                first = false;
            }

            match value {
                Value::String(s) => {
                    builder.push(field).push(" = ").push_bind(s);
                }
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        builder.push(field).push(" = ").push_bind(i);
                    } else if let Some(f) = n.as_f64() {
                        builder.push(field).push(" = ").push_bind(f);
                    }
                }
                Value::Bool(b) => {
                    builder.push(field).push(" = ").push_bind(b);
                }
                _ => continue, // 忽略复杂类型
            }
        }
    }

    let query = builder.build_query_as::<T>();
    let rows = query.fetch_all(pool).await?;
    Ok(rows)
}

/// 通用查询函数，执行传入的 sql语句 并返回结果集
pub async fn run_query<'q, T>(
    pool: &PgPool, // PostgreSQL 连接池
    query: QueryAs<'q, Postgres, T, sqlx::postgres::PgArguments>,
) -> std::result::Result<Vec<T>, anyhow::Error>
where
    for<'r> T: FromRow<'r, PgRow> + Send + Unpin,
{
    let rows = query
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow::anyhow!("query error: {:?}", e))?;
    Ok(rows)
}
