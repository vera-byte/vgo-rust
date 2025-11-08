# 数据库示例结构与用法

本目录提供三种数据库的标准示例结构，每个示例包含 model 层与 repo 层，并通过 `Repository` Trait 实现通用的 CRUD：

目录结构：
- `examples/db/sqlite`：使用内存数据库，可直接运行
- `examples/db/mysql`：通过环境变量 `DATABASE_URL` 连接
- `examples/db/postgresql`：通过环境变量 `DATABASE_URL` 连接

统一模型：示例使用 `BaseSysConf { id, name, value }`，并可选包含 `v::model::base::BaseModel`。

通用仓库 Trait：在 `v::repo::Repository<T, PK>` 中定义：
- `create(&self, model: &T) -> Result<u64>`
- `read_one(&self, pk: PK) -> Result<Option<T>>`
- `read_all(&self) -> Result<Vec<T>>`
- `update(&self, model: &T) -> Result<u64>`
- `delete(&self, pk: PK) -> Result<u64>`
- `page(&self, limit: i64, offset: i64) -> Result<Vec<T>>`

运行示例：
- SQLite：`cargo run --example db/sqlite`
- MySQL：`export DATABASE_URL="mysql://user:pass@127.0.0.1:3306/vgo" && cargo run --example db/mysql`
- Postgres：`export DATABASE_URL="postgres://user:pass@127.0.0.1:5432/vgo" && cargo run --example db/postgresql`

测试：
- 已提供集成测试：`v/tests/sqlite_repo.rs`，使用内存 SQLite 验证完整 CRUD。

注意：示例使用 `sqlx::query` 动态 SQL，无需启用 `query!` 宏或 offline 模式。