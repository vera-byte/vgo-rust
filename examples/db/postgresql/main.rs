mod model;

use anyhow::{Ok, Result};
use model::BaseSysConf;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<()> {
    println!("[info] postgres example using query_as! macro");
    // 使用环境变量 DATABASE_URL 构建 Postgres 连接池
    let database_url = std::env::var("DATABASE_URL")?;
    let pool: sqlx::Pool<sqlx::Postgres> = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // 直接使用 BaseSysConf（宏已派生 FromRow 并处理字段重命名）
    let configs = sqlx::query_as::<_, BaseSysConf>(
        r#"
        SELECT
            id,
            tenant_id,
            c_key,
            c_value,
            created_at,
            updated_at,
            deleted_at
        FROM base_sys_conf
        "#,
    )
    .fetch_all(&pool)
    .await?;

    println!("[info] fetched {} rows", configs.len());
    for item in &configs {
        println!("{:?}", item);
    }

    Ok(())

    // for row in rows {
    //     let json_text: String = row.try_get("json_text")?;
    //     let item: BaseSysConf = serde_json::from_str(&json_text)?;
    //     configs.push(item);
    // }

    // println!(
    //     "[info] fetched {} rows from '{}'",
    //     configs.len(),
    //     table_name
    // );
    // for item in &configs {
    //     println!("{:?}", item);
    // }
}
