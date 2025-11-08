mod model;
mod repo;

use anyhow::Result;
use model::{BaseSysConf, BaseSysConf2};
use repo::{create_table_for_model, insert_demo_for_model, query_all_for_model};
use serde_json::Value as JsonValue;
use sqlx::Row;
use v::{DatabaseManager, Model};

/// 在指定表中创建一条记录（name, value），返回影响行数
async fn create_one<T: Model>(name: &str, value: &str) -> Result<u64> {
    let (pool, table, _group) = DatabaseManager::get_any_pool::<T>().await?;
    let sql = format!("INSERT INTO \"{}\" (name, value) VALUES ($1, $2)", table);
    let result = sqlx::query(&sql)
        .bind(name)
        .bind(value)
        .execute(&pool)
        .await?;
    Ok(result.rows_affected() as u64)
}

/// 读取一条记录（按 name），返回 JSON 对象（包含 id/name/value），不存在返回 None
async fn read_one_by_name<T: Model>(name: &str) -> Result<Option<JsonValue>> {
    let (pool, table, _group) = DatabaseManager::get_any_pool::<T>().await?;
    let sql = format!(
        "SELECT id, name, value FROM \"{}\" WHERE name = $1 LIMIT 1",
        table
    );
    let row = sqlx::query(&sql).bind(name).fetch_optional(&pool).await?;
    if let Some(row) = row {
        let mut obj = serde_json::Map::new();
        obj.insert(
            "id".to_string(),
            JsonValue::from(row.try_get::<i32, _>("id")?),
        );
        obj.insert(
            "name".to_string(),
            JsonValue::from(row.try_get::<String, _>("name")?),
        );
        obj.insert(
            "value".to_string(),
            JsonValue::from(row.try_get::<String, _>("value")?),
        );
        Ok(Some(JsonValue::Object(obj)))
    } else {
        Ok(None)
    }
}

/// 更新一条记录（按 name），返回影响行数
async fn update_one_by_name<T: Model>(name: &str, value: &str) -> Result<u64> {
    let (pool, table, _group) = DatabaseManager::get_any_pool::<T>().await?;
    let sql = format!("UPDATE \"{}\" SET value = $1 WHERE name = $2", table);
    let result = sqlx::query(&sql)
        .bind(value)
        .bind(name)
        .execute(&pool)
        .await?;
    Ok(result.rows_affected() as u64)
}

/// 删除一条记录（按 name），返回影响行数
async fn delete_one_by_name<T: Model>(name: &str) -> Result<u64> {
    let (pool, table, _group) = DatabaseManager::get_any_pool::<T>().await?;
    let sql = format!("DELETE FROM \"{}\" WHERE name = $1", table);
    let result = sqlx::query(&sql).bind(name).execute(&pool).await?;
    Ok(result.rows_affected() as u64)
}

/// 分页读取，返回 JSON 列表（按 id 排序）
async fn page<T: Model>(limit: i64, offset: i64) -> Result<Vec<JsonValue>> {
    let (pool, table, _group) = DatabaseManager::get_any_pool::<T>().await?;
    let sql = format!(
        "SELECT id, name, value FROM \"{}\" ORDER BY id LIMIT $1 OFFSET $2",
        table
    );
    let rows = sqlx::query(&sql)
        .bind(limit)
        .bind(offset)
        .fetch_all(&pool)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let mut obj = serde_json::Map::new();
        obj.insert(
            "id".to_string(),
            JsonValue::from(row.try_get::<i32, _>("id")?),
        );
        obj.insert(
            "name".to_string(),
            JsonValue::from(row.try_get::<String, _>("name")?),
        );
        obj.insert(
            "value".to_string(),
            JsonValue::from(row.try_get::<String, _>("value")?),
        );
        out.push(JsonValue::Object(obj));
    }
    Ok(out)
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("[info] postgres group-based example using repo helpers");

    // 可选：如需使用通用 AnyPool，请使用 `DatabaseManager::get_db_pool::<Model>()`
    // 但为避免未安装驱动或连接失败影响演示，此处不启用。

    // default 组：base_sys_conf（建表 -> 插入演示数据 -> CRUD 测试）
    match create_table_for_model::<BaseSysConf>().await {
        Ok(_) => {
            if let Err(e) = insert_demo_for_model::<BaseSysConf>().await {
                eprintln!("[warn] insert default failed: {}", e);
            } else {
                match query_all_for_model::<BaseSysConf>().await {
                    Ok(rows) => {
                        println!("[info] query default/base_sys_conf => {} rows", rows.len());
                        println!("{}", serde_json::to_string_pretty(&rows)?);
                    }
                    Err(e) => eprintln!("[warn] query default failed: {}", e),
                }

                // Create：新增一条记录
                let created = create_one::<BaseSysConf>("extra", "value").await?;
                println!(
                    "[info] create default/base_sys_conf => affected {}",
                    created
                );

                // Read：按 name 读取一条
                let one = read_one_by_name::<BaseSysConf>("extra").await?;
                println!(
                    "[info] read_one default/base_sys_conf(extra) => {}",
                    serde_json::to_string_pretty(&one)?
                );

                // Update：修改记录值
                let updated = update_one_by_name::<BaseSysConf>("language", "rust-async").await?;
                println!(
                    "[info] update default/base_sys_conf(language) => affected {}",
                    updated
                );

                // Page：分页读取
                let page_rows = page::<BaseSysConf>(2, 0).await?;
                println!(
                    "[info] page default/base_sys_conf(limit=2,offset=0) => {} rows",
                    page_rows.len()
                );
                println!("{}", serde_json::to_string_pretty(&page_rows)?);

                // Delete：删除记录
                let deleted = delete_one_by_name::<BaseSysConf>("extra").await?;
                println!(
                    "[info] delete default/base_sys_conf(extra) => affected {}",
                    deleted
                );
            }
        }
        Err(e) => eprintln!("[warn] connect or create default failed: {}", e),
    }

    // test 组：base_sys_conf2（同样执行 CRUD 测试）
    match create_table_for_model::<BaseSysConf2>().await {
        Ok(_) => {
            if let Err(e) = insert_demo_for_model::<BaseSysConf2>().await {
                eprintln!("[warn] insert test failed: {}", e);
            } else {
                match query_all_for_model::<BaseSysConf2>().await {
                    Ok(rows) => {
                        println!("[info] query test/base_sys_conf2 => {} rows", rows.len());
                        println!("{}", serde_json::to_string_pretty(&rows)?);
                    }
                    Err(e) => eprintln!("[warn] query test failed: {}", e),
                }

                let created = create_one::<BaseSysConf2>("extra", "value").await?;
                println!("[info] create test/base_sys_conf2 => affected {}", created);

                let one = read_one_by_name::<BaseSysConf2>("extra").await?;
                println!(
                    "[info] read_one test/base_sys_conf2(extra) => {}",
                    serde_json::to_string_pretty(&one)?
                );

                let updated =
                    update_one_by_name::<BaseSysConf2>("another_site", "vgo-test-updated").await?;
                println!(
                    "[info] update test/base_sys_conf2(another_site) => affected {}",
                    updated
                );

                let page_rows = page::<BaseSysConf2>(2, 0).await?;
                println!(
                    "[info] page test/base_sys_conf2(limit=2,offset=0) => {} rows",
                    page_rows.len()
                );
                println!("{}", serde_json::to_string_pretty(&page_rows)?);

                let deleted = delete_one_by_name::<BaseSysConf2>("extra").await?;
                println!(
                    "[info] delete test/base_sys_conf2(extra) => affected {}",
                    deleted
                );
            }
        }
        Err(e) => eprintln!("[warn] connect or create test failed: {}", e),
    }

    println!("[info] done");
    Ok(())
}
