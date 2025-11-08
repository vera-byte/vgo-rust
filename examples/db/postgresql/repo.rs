use anyhow::Result;
use serde_json::Value as JsonValue;
use sqlx::{Column, Row};
use v::{DatabaseManager, Model};

/// 为指定模型创建表（PostgreSQL），字段：id SERIAL, name TEXT, value TEXT
pub async fn create_table_for_model<T: Model>() -> Result<()> {
    let (pool, table, _group) = DatabaseManager::get_any_pool::<T>().await?;
    let sql = format!(
        r#"
        CREATE TABLE IF NOT EXISTS "{table}" (
            id SERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            value TEXT NOT NULL
        );
        "#
    );
    sqlx::query(&sql).execute(&pool).await?;
    Ok(())
}

/// 为指定模型插入两条示例数据
pub async fn insert_demo_for_model<T: Model>() -> Result<()> {
    let (pool, table, _group) = DatabaseManager::get_any_pool::<T>().await?;
    let delete_sql = format!("DELETE FROM \"{}\"", table);
    sqlx::query(&delete_sql).execute(&pool).await?;

    let insert_sql = format!("INSERT INTO \"{}\" (name, value) VALUES ($1, $2)", table);
    sqlx::query(&insert_sql)
        .bind("site_name")
        .bind("vgo")
        .execute(&pool)
        .await?;
    sqlx::query(&insert_sql)
        .bind("language")
        .bind("rust")
        .execute(&pool)
        .await?;
    Ok(())
}

/// 查询指定模型的全部数据，返回 JSON 列表
pub async fn query_all_for_model<T: Model>() -> Result<Vec<JsonValue>> {
    let (pool, table, _group) = DatabaseManager::get_any_pool::<T>().await?;

    let sql = format!("SELECT * FROM \"{}\"", table);
    let rows = sqlx::query(&sql).fetch_all(&pool).await?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let mut obj = serde_json::Map::new();
        for col in row.columns() {
            let key = col.name();
            // 尝试常见类型的读取；读取失败则置为 Null
            let value = if let Ok(v) = row.try_get::<i64, _>(key) {
                JsonValue::from(v)
            } else if let Ok(v) = row.try_get::<i32, _>(key) {
                JsonValue::from(v)
            } else if let Ok(v) = row.try_get::<f64, _>(key) {
                JsonValue::from(v)
            } else if let Ok(v) = row.try_get::<String, _>(key) {
                JsonValue::from(v)
            } else if let Ok(v) = row.try_get::<Option<String>, _>(key) {
                match v {
                    Some(s) => JsonValue::from(s),
                    None => JsonValue::Null,
                }
            } else if let Ok(v) = row.try_get::<bool, _>(key) {
                JsonValue::from(v)
            } else {
                JsonValue::Null
            };
            obj.insert(key.to_string(), value);
        }
        out.push(JsonValue::Object(obj));
    }
    Ok(out)
}
