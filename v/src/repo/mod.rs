use crate::db::error::DbError;
type Result<T> = std::result::Result<T, DbError>;
use async_trait::async_trait;

/// 通用仓库 Trait，约定标准 CRUD 操作。
/// 该 Trait 不依赖具体数据库类型，具体实现可使用 MySQL/Postgres/SQLite 的连接池。
#[async_trait]
pub trait Repository<T, PK> {
    /// 创建记录，返回影响行数或主键值（实现决定返回语义）。
    async fn create(&self, model: &T) -> Result<u64>;

    /// 读取一条记录（按主键）。
    async fn read_one(&self, pk: PK) -> Result<Option<T>>;

    /// 读取所有记录。
    async fn read_all(&self) -> Result<Vec<T>>;

    /// 更新记录（通常按主键）。
    async fn update(&self, model: &T) -> Result<u64>;

    /// 删除记录（按主键）。
    async fn delete(&self, pk: PK) -> Result<u64>;

    /// 分页读取。
    async fn page(&self, limit: i64, offset: i64) -> Result<Vec<T>>;
}
