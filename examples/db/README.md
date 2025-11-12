# 数据库示例结构与用法

本目录提供三种数据库的标准示例结构，使用 Diesel（同步 ORM，结合 r2d2 连接池）。示例包含简单的模型与 CRUD：

目录结构：
- `examples/db/sqlite`：使用内存数据库，可直接运行
- `examples/db/mysql`：通过环境变量 `DATABASE_URL` 连接
- `examples/db/postgresql`：通过环境变量 `DATABASE_URL` 连接

统一模型：示例使用 `BaseSysConf { id, name, value }`，通过 `#[derive(Queryable, Insertable)]` 与内联 `table!` 宏映射。

通用仓库 Trait：在 `v::repo::Repository<T, PK>` 中定义：
- `create(&self, model: &T) -> Result<u64>`
- `read_one(&self, pk: PK) -> Result<Option<T>>`
- `read_all(&self) -> Result<Vec<T>>`
- `update(&self, model: &T) -> Result<u64>`
- `delete(&self, pk: PK) -> Result<u64>`
- `page(&self, limit: i64, offset: i64) -> Result<Vec<T>>`

运行示例：
- SQLite：`cargo run --example db_sqlite`
- MySQL：`export DATABASE_URL="mysql://user:pass@127.0.0.1:3306/vgo" && cargo run --example db_mysql`
- Postgres：`export DATABASE_URL="postgres://user:pass@127.0.0.1:5432/vgo" && cargo run --example db_postgresql`

注意：Diesel 为同步库，如在异步上下文中使用，请通过 `tokio::task::spawn_blocking` 包裹阻塞调用。

本示例不再使用 sqlx，全部改为 Diesel；建表使用 `diesel::sql_query`，其余 CRUD 使用 Diesel DSL。