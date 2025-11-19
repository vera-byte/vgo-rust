# v::db (PostgreSQL via sqlx)

简体中文 / English

## 概述 / Overview
- 提供基于 `sqlx` 的 PostgreSQL 连接池与查询构建器。
- 自动根据模型中的分库组路由连接（`DbModel::table_group`）。
- 支持基本 CRUD、事务、查询缓存与批量插入。

## 快速开始 / Quick Start
1. 配置环境变量（或 `config/*.toml`）：
   - `V_DATABASE_<GROUP>_TYPE=postgresql`
   - `V_DATABASE_<GROUP>_URL=postgres://user:pass@host:port/db`（或分别设置 `host/port/user/pass/name/maxOpen`）
2. 为模型实现 `DbModel`（或使用宏）：
```rust
use v::db::model::DbModel;

struct MyModel;
impl DbModel for MyModel {
  fn table_name() -> &'static str { "my_table" }
  fn table_group() -> &'static str { "default" }
}
```
3. 使用查询构建器：
```rust
use v::db::query::QueryPg;

let rows = QueryPg::<MyModel>::new().await?
  .select(&["id", "name"]) // 默认 `*`
  .where_eq("status", 1)
  .order_by("id DESC")
  .limit(10)
  .enable_cache(std::time::Duration::from_secs(30))
  .fetch_all_json().await?;
```

## 事务 / Transactions
```rust
use v::db::connection::{begin_tx};
use v::db::model::pool_for;

let pool = pool_for::<MyModel>().await?;
let mut tx = begin_tx(&pool).await?;
sqlx::query("SELECT 1").execute(&mut *tx).await?;
tx.commit().await?;
```

## 批量插入 / Batch Insert
```rust
use v::db::query::QueryPg;

let items = vec![
  serde_json::json!({"name":"a","status":1}),
  serde_json::json!({"name":"b","status":1}),
];
let n = QueryPg::<MyModel>::new().await?.insert_many_json(&items).await?;
```

## 注意事项 / Notes
- 查询缓存为简单 TTL 缓存，仅针对构建器生成的 `SELECT` 有效。
- `insert_one_spec` 依赖 `ModelSpec::columns` 进行字段绑定；建议为复杂模型实现该 Trait。
- 为获得编译期类型校验，可在未来接入 `sqlx::query_as!` 宏与离线检查。

## 扩展性 / Extensibility
- 接口已预留以支持其他数据库类型（可在 `connection.rs` 增加实现）。
- 查询构建器为可插拔设计，可扩展更多语义方法（如 `where_in`、`join`）。

