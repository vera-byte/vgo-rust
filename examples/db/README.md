# 数据库示例结构与用法

本目录提供三种数据库的标准示例结构，使用 Diesel（同步 ORM，结合 r2d2 连接池）。示例包含简单的模型与 CRUD：

## 目录结构

- `examples/db/sqlite`：使用内存数据库，可直接运行
- `examples/db/mysql`：通过配置文件或环境变量连接
- `examples/db/postgresql`：通过配置文件连接，支持重试与健康检查

## 模型定义

### SQLite/MySQL 示例模型
简化模型：`BaseSysConf { id, name, value }`

### PostgreSQL 示例模型
完整模型：`BaseSysConf { id, tenant_id, created_at, updated_at, deleted_at, c_key, c_value }`

所有模型通过 `#[derive(Queryable, Insertable)]` 与内联 `table!` 宏映射。

## 通用仓库 Trait

在 `v::repo::Repository<T, PK>` 中定义：
- `create(&self, model: &T) -> Result<u64>` - 创建记录
- `read_one(&self, pk: PK) -> Result<Option<T>>` - 按主键读取
- `read_all(&self) -> Result<Vec<T>>` - 读取所有记录
- `update(&self, model: &T) -> Result<u64>` - 更新记录
- `delete(&self, pk: PK) -> Result<u64>` - 删除记录
- `page(&self, limit: i64, offset: i64) -> Result<Vec<T>>` - 分页读取

## 运行示例

从 workspace 根目录运行：

```bash
# SQLite 示例（使用内存数据库）
cargo run -p vgo-examples --example db_sqlite

# MySQL 示例（需要配置 DATABASE_URL 环境变量）
export DATABASE_URL="mysql://user:pass@127.0.0.1:3306/vgo"
cargo run -p vgo-examples --example db_mysql --features mysql_backend

# PostgreSQL 示例（使用配置文件 config/default.toml）
cargo run -p vgo-examples --example db_postgresql
```

## 配置说明

PostgreSQL 示例使用配置文件 `config/default.toml` 中的 `database.default` 分组：

```toml
[database.default]
type = "postgresql"
host = "127.0.0.1"
port = "5432"
user = "vgo_rust_master"
pass = "wRJksJcYWsRBbhHw"
name = "vgo_rust_master"
maxOpen = 100
```

可通过环境变量覆盖：
```bash
export V_DATABASE_DEFAULT_HOST="10.0.0.200"
export V_DATABASE_DEFAULT_MAXOPEN="10"
```

## 注意事项

1. **Diesel 为同步库**：在异步上下文中使用时，请通过 `tokio::task::spawn_blocking` 包裹阻塞调用
2. **连接池管理**：使用 `DatabaseManager` 自动管理连接池，支持健康检查与自动重连
3. **Model Trait**：模型需实现 `v::db::database::Model` trait，提供 `table_name()` 和 `group_name()`

## 技术栈

- **ORM**: Diesel 2.1+
- **连接池**: r2d2
- **异步运行时**: Tokio
- **建表**: `diesel::sql_query`
- **CRUD**: Diesel DSL