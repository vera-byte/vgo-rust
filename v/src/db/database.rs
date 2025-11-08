use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use sqlx::any::AnyPoolOptions;
use sqlx::AnyPool;
use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::sync::RwLock;

use crate::comm::config::get_global_config_manager;

lazy_static! {
    /// PostgreSQL 连接池缓存，按 group_name 管理
    static ref POSTGRES_POOLS: RwLock<HashMap<String, PgPool>> = RwLock::new(HashMap::new());
    /// MySQL 连接池缓存
    static ref MYSQL_POOLS: RwLock<HashMap<String, MySqlPool>> = RwLock::new(HashMap::new());
    /// SQLite 连接池缓存
    static ref SQLITE_POOLS: RwLock<HashMap<String, SqlitePool>> = RwLock::new(HashMap::new());
    /// Any 连接池缓存（动态类型），用于通用 get_db_pool<T>()
    static ref ANY_POOLS: RwLock<HashMap<String, AnyPool>> = RwLock::new(HashMap::new());

    /// 已经启动健康检查的 group 记录（避免重复）
    static ref HEALTH_GROUPS: RwLock<HashSet<String>> = RwLock::new(HashSet::new());
}

/// 模型约定：提供表名与分组名常量访问
/// 由宏 `#[v_macros::model(table_name = "...", group_name = "...")]` 生成常量后，可手动实现该 Trait。
pub trait Model {
    fn table_name() -> &'static str;
    fn group_name() -> &'static str;
}

/// 数据库管理器：按 group 懒加载并缓存连接池；支持健康检查与断线重建。
pub struct DatabaseManager;

impl DatabaseManager {
    /// 根据模型类型返回对应组的通用连接池，并同时读取表名与组名。
    /// 返回值为 `(AnyPool, TABLE_NAME, TABLE_GROUP)`，便于调用方直接构造 SQL。
    pub async fn get_any_pool<T: Model>() -> Result<(AnyPool, &'static str, &'static str)> {
        let group = T::group_name();
        let table = T::table_name();
        let pool = Self::get_any_pool_by_group(group).await?;
        Ok((pool, table, group))
    }
    /// 通用：根据模型的 `group_name` 返回对应的 AnyPool（惰性初始化）
    /// 通过配置项 `database.<group>.type` 决定具体数据库（postgresql/mysql/sqlite）。
    pub async fn get_db_pool<T: Model>() -> Result<AnyPool> {
        let group = T::group_name();
        Self::get_any_pool_by_group(group).await
    }

    /// 构建通用 AnyPool，依据 `database.<group>.type` 与其它配置拼接 URL
    async fn build_any_pool(group: &str) -> Result<AnyPool> {
        let mgr = get_global_config_manager()?;

        let typ: String = mgr
            .get_or(&format!("database.{}.type", group), "postgresql".to_string())
            .to_lowercase();

        // 优先读取显式 URL，否则按类型拼接默认 URL
        let url_opt: Option<String> = mgr
            .get(&format!("database.{}.url", group))
            .ok();

        let max_open: u32 = mgr
            .get(&format!("database.{}.maxOpen", group))
            .map(|v: i64| v as u32)
            .unwrap_or(10);

        // 构建 AnyPool：按类型拼接 URL，直接使用 AnyPoolOptions 连接
        let url = match typ.as_str() {
            "postgresql" | "postgres" => match url_opt {
                Some(u) => u,
                None => {
                    let host: String = mgr.get_or(&format!("database.{}.host", group), "127.0.0.1".to_string());
                    let port: String = mgr.get_or(&format!("database.{}.port", group), "5432".to_string());
                    let user: String = mgr.get_or(&format!("database.{}.user", group), "postgres".to_string());
                    let pass: String = mgr.get_or(&format!("database.{}.pass", group), "".to_string());
                    let name: String = mgr.get_or(&format!("database.{}.name", group), "postgres".to_string());
                    Self::build_postgres_url(&host, &port, &user, &pass, &name)
                }
            },
            "mysql" => match url_opt {
                Some(u) => u,
                None => {
                    let host: String = mgr.get_or(&format!("database.{}.host", group), "127.0.0.1".to_string());
                    let port: String = mgr.get_or(&format!("database.{}.port", group), "3306".to_string());
                    let user: String = mgr.get_or(&format!("database.{}.user", group), "root".to_string());
                    let pass: String = mgr.get_or(&format!("database.{}.pass", group), "".to_string());
                    let name: String = mgr.get_or(&format!("database.{}.name", group), "mysql".to_string());
                    Self::build_mysql_url(&host, &port, &user, &pass, &name)
                }
            },
            "sqlite" => match url_opt {
                Some(u) => u,
                None => "sqlite::memory:".to_string(),
            },
            other => return Err(anyhow!("不支持的数据库类型: {} (group={})", other, group)),
        };

        // 使用 AnyPool 前需要安装编译进来的驱动，否则会出现 "No drivers installed" 的运行时错误
        sqlx::any::install_default_drivers();

        let any_pool = AnyPoolOptions::new()
            .max_connections(max_open)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&url)
            .await?;

        println!(
            "[db] any pool initialized for group '{}' (type={}) maxOpen={}",
            group, typ, max_open
        );
        Ok(any_pool)
    }
    /// 通用：按组获取 AnyPool（动态数据库类型），用于不便引入模型类型的场景。
    pub async fn get_any_pool_by_group(group: &str) -> Result<AnyPool> {
        if let Some(pool) = {
            let r = (&*ANY_POOLS).read().await;
            r.get(group).cloned()
        } {
            if let Err(e) = Self::check_any_health(&pool).await {
                println!("[db] any pool unhealthy for group '{}': {}; rebuilding", group, e);
                let new_pool = Self::build_any_pool(group).await?;
                let mut w = (&*ANY_POOLS).write().await;
                w.insert(group.to_string(), new_pool.clone());
                return Ok(new_pool);
            }
            return Ok(pool);
        }
        let pool = Self::build_any_pool(group).await?;
        {
            let mut w = (&*ANY_POOLS).write().await;
            w.insert(group.to_string(), pool.clone());
        }
        Self::start_group_health_task_once(group).await;
        Ok(pool)
    }
    // 已移除的按组获取 PostgreSQL 连接池片段，统一使用 AnyPool 接口。

    /// 构建 PostgreSQL 连接池（使用 connect_lazy，避免在无数据库时阻塞/失败）
    async fn build_postgres_pool(group: &str) -> Result<PgPool> {
        let mgr = get_global_config_manager()?;

        // 类型校验：必须为 postgresql
        let typ: String = mgr
            .get_or(&format!("database.{}.type", group), "postgresql".to_string());
        if typ.to_lowercase() != "postgresql" {
            return Err(anyhow!(
                "数据库组 '{}' 类型非 postgresql: {}",
                group, typ
            ));
        }

        // 数据源：优先读取显式 URL，否则拼接
        let url: Option<String> = mgr
            .get(&format!("database.{}.url", group))
            .ok();
        let url = match url {
            Some(u) => u,
            None => {
                let host: String = mgr.get_or(&format!("database.{}.host", group), "127.0.0.1".to_string());
                let port: String = mgr.get_or(&format!("database.{}.port", group), "5432".to_string());
                let user: String = mgr.get_or(&format!("database.{}.user", group), "postgres".to_string());
                let pass: String = mgr.get_or(&format!("database.{}.pass", group), "".to_string());
                let name: String = mgr.get_or(&format!("database.{}.name", group), "postgres".to_string());
                Self::build_postgres_url(&host, &port, &user, &pass, &name)
            }
        };

        let max_open: u32 = mgr
            .get(&format!("database.{}.maxOpen", group))
            .map(|v: i64| v as u32)
            .unwrap_or(10);

        let options = PgPoolOptions::new()
            .max_connections(max_open)
            .acquire_timeout(Duration::from_secs(5));

        // 使用 lazy 连接，避免测试或启动阶段必须连通数据库
        let pool = options.connect_lazy(&url)?;
        println!(
            "[db] postgres pool initialized (lazy) for group '{}' with maxOpen={}",
            group, max_open
        );
        Ok(pool)
    }
    // 已移除的按组获取 MySQL 连接池片段，统一使用 AnyPool 接口。

    async fn build_mysql_pool(group: &str) -> Result<MySqlPool> {
        let mgr = get_global_config_manager()?;
        let typ: String = mgr.get_or(&format!("database.{}.type", group), "mysql".to_string());
        if typ.to_lowercase() != "mysql" {
            return Err(anyhow!("数据库组 '{}' 类型非 mysql: {}", group, typ));
        }
        let url: Option<String> = mgr.get(&format!("database.{}.url", group)).ok();
        let url = match url {
            Some(u) => u,
            None => {
                let host: String = mgr.get_or(&format!("database.{}.host", group), "127.0.0.1".to_string());
                let port: String = mgr.get_or(&format!("database.{}.port", group), "3306".to_string());
                let user: String = mgr.get_or(&format!("database.{}.user", group), "root".to_string());
                let pass: String = mgr.get_or(&format!("database.{}.pass", group), "".to_string());
                let name: String = mgr.get_or(&format!("database.{}.name", group), "mysql".to_string());
                Self::build_mysql_url(&host, &port, &user, &pass, &name)
            }
        };
        let max_open: u32 = mgr
            .get(&format!("database.{}.maxOpen", group))
            .map(|v: i64| v as u32)
            .unwrap_or(10);
        let options = MySqlPoolOptions::new()
            .max_connections(max_open)
            .acquire_timeout(Duration::from_secs(5));
        let pool = options.connect_lazy(&url)?;
        println!("[db] mysql pool initialized (lazy) for group '{}' with maxOpen={}", group, max_open);
        Ok(pool)
    }

    fn build_mysql_url(host: &str, port: &str, user: &str, pass: &str, name: &str) -> String {
        let enc_user = urlencoding::encode(user);
        let enc_pass = urlencoding::encode(pass);
        format!("mysql://{}:{}@{}:{}/{}", enc_user, enc_pass, host, port, name)
    }

    // 已移除的按组获取 SQLite 连接池片段，统一使用 AnyPool 接口。

    async fn build_sqlite_pool(group: &str) -> Result<SqlitePool> {
        let mgr = get_global_config_manager()?;
        let typ: String = mgr.get_or(&format!("database.{}.type", group), "sqlite".to_string());
        if typ.to_lowercase() != "sqlite" {
            return Err(anyhow!("数据库组 '{}' 类型非 sqlite: {}", group, typ));
        }
        let url: Option<String> = mgr.get(&format!("database.{}.url", group)).ok();
        let url = match url {
            Some(u) => u,
            None => {
                // 默认使用内存库，避免文件路径问题；生产可通过 `database.<group>.url` 指定文件路径
                "sqlite::memory:".to_string()
            }
        };
        let max_open: u32 = mgr
            .get(&format!("database.{}.maxOpen", group))
            .map(|v: i64| v as u32)
            .unwrap_or(5);
        let options = SqlitePoolOptions::new()
            .max_connections(max_open)
            .acquire_timeout(Duration::from_secs(5));
        let pool = options.connect(&url).await?; // Sqlite 不支持 connect_lazy
        println!("[db] sqlite pool initialized for group '{}' with maxOpen={}", group, max_open);
        Ok(pool)
    }

    /// 构建 PostgreSQL URL，对用户名与密码进行编码
    fn build_postgres_url(host: &str, port: &str, user: &str, pass: &str, name: &str) -> String {
        let enc_user = urlencoding::encode(user);
        let enc_pass = urlencoding::encode(pass);
        format!("postgres://{}:{}@{}:{}/{}", enc_user, enc_pass, host, port, name)
    }

    /// 轻量健康检查：执行 `SELECT 1`，失败返回错误
    pub async fn check_postgres_health(pool: &PgPool) -> Result<()> {
        // 使用简单查询作为健康检查；注意：lazy 连接在首次实际查询时建立连接
        sqlx::query("SELECT 1").execute(pool).await.map(|_| ()).map_err(|e| {
            anyhow!("postgres health check failed: {}", e)
        })
    }

    pub async fn check_mysql_health(pool: &MySqlPool) -> Result<()> {
        sqlx::query("SELECT 1").execute(pool).await.map(|_| ()).map_err(|e| {
            anyhow!("mysql health check failed: {}", e)
        })
    }

    pub async fn check_sqlite_health(pool: &SqlitePool) -> Result<()> {
        sqlx::query("SELECT 1").execute(pool).await.map(|_| ()).map_err(|e| {
            anyhow!("sqlite health check failed: {}", e)
        })
    }

    pub async fn check_any_health(pool: &AnyPool) -> Result<()> {
        sqlx::query("SELECT 1").execute(pool).await.map(|_| ()).map_err(|e| {
            anyhow!("any health check failed: {}", e)
        })
    }

    /// 手动触发分组健康检查；必要时自动重建（按类型选择具体实现）
    pub async fn ensure_group_healthy(group: &str) -> Result<()> {
        let mgr = get_global_config_manager()?;
        let typ: String = mgr
            .get_or(&format!("database.{}.type", group), "postgresql".to_string())
            .to_lowercase();

        match typ.as_str() {
            "postgresql" | "postgres" => {
                let pool = {
                    let r = POSTGRES_POOLS.read().await;
                    r.get(group).cloned()
                };
                let Some(pool) = pool else {
                    let pool = Self::build_postgres_pool(group).await?;
                    let mut w = POSTGRES_POOLS.write().await;
                    w.insert(group.to_string(), pool);
                    return Ok(());
                };
                if let Err(e) = Self::check_postgres_health(&pool).await {
                    println!("[db] health failed for group '{}': {}; rebuilding", group, e);
                    let new_pool = Self::build_postgres_pool(group).await?;
                    let mut w = POSTGRES_POOLS.write().await;
                    w.insert(group.to_string(), new_pool);
                }
                Ok(())
            }
            "mysql" => {
                let pool = {
                    let r = MYSQL_POOLS.read().await;
                    r.get(group).cloned()
                };
                let Some(pool) = pool else {
                    let pool = Self::build_mysql_pool(group).await?;
                    let mut w = MYSQL_POOLS.write().await;
                    w.insert(group.to_string(), pool);
                    return Ok(());
                };
                if let Err(e) = Self::check_mysql_health(&pool).await {
                    println!("[db] health failed for group '{}': {}; rebuilding", group, e);
                    let new_pool = Self::build_mysql_pool(group).await?;
                    let mut w = MYSQL_POOLS.write().await;
                    w.insert(group.to_string(), new_pool);
                }
                Ok(())
            }
            "sqlite" => {
                let pool = {
                    let r = SQLITE_POOLS.read().await;
                    r.get(group).cloned()
                };
                let Some(pool) = pool else {
                    let pool = Self::build_sqlite_pool(group).await?;
                    let mut w = SQLITE_POOLS.write().await;
                    w.insert(group.to_string(), pool);
                    return Ok(());
                };
                if let Err(e) = Self::check_sqlite_health(&pool).await {
                    println!("[db] health failed for group '{}': {}; rebuilding", group, e);
                    let new_pool = Self::build_sqlite_pool(group).await?;
                    let mut w = SQLITE_POOLS.write().await;
                    w.insert(group.to_string(), new_pool);
                }
                Ok(())
            }
            other => Err(anyhow!("不支持的数据库类型: {} (group={})", other, group)),
        }
    }

    /// 启动周期性健康检查（每 60s），避免重复启动
    async fn start_group_health_task_once(group: &str) {
        let already = {
            let r = HEALTH_GROUPS.read().await;
            r.contains(group)
        };
        if already {
            return;
        }
        {
            let mut w = HEALTH_GROUPS.write().await;
            w.insert(group.to_string());
        }

        let group_name = group.to_string();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                if let Err(e) = DatabaseManager::ensure_group_healthy(&group_name).await {
                    println!("[db] periodic health check error for '{}': {}", group_name, e);
                }
            }
        });
    }

    /// 仅用于测试：当前缓存池数量
    #[cfg(test)]
    pub async fn pool_count_postgres() -> usize { POSTGRES_POOLS.read().await.len() }
    #[cfg(test)]
    pub async fn pool_count_mysql() -> usize { MYSQL_POOLS.read().await.len() }
    #[cfg(test)]
    pub async fn pool_count_sqlite() -> usize { SQLITE_POOLS.read().await.len() }
    #[cfg(test)]
    pub async fn pool_count_any() -> usize { (&*ANY_POOLS).read().await.len() }

    /// 仅用于测试：重置池缓存
    #[cfg(test)]
    pub async fn reset_postgres_pools() {
        POSTGRES_POOLS.write().await.clear();
        HEALTH_GROUPS.write().await.clear();
    }
    #[cfg(test)]
    pub async fn reset_mysql_pools() { MYSQL_POOLS.write().await.clear(); }
    #[cfg(test)]
    pub async fn reset_sqlite_pools() { SQLITE_POOLS.write().await.clear(); }
    #[cfg(test)]
    pub async fn reset_any_pools() { (&*ANY_POOLS).write().await.clear(); }
}

#[cfg(test)]
mod tests {
    use super::DatabaseManager;

    #[test]
    fn test_build_postgres_url() {
        let url = DatabaseManager::build_postgres_url("10.0.0.200", "5432", "user@x", "p@ss:wd", "vgo");
        assert!(url.starts_with("postgres://"));
        assert!(url.contains("10.0.0.200:5432/vgo"));
        assert!(url.contains("user%40x"));
        assert!(url.contains("p%40ss%3Awd"));
    }

    #[tokio::test]
    async fn test_postgres_pool_lazy_init_and_cache() {
        // 使用环境变量注入配置（通过 ConfigManager 的 Env 源，前缀 V_，下划线分隔）
        std::env::set_var("V_DATABASE_DEFAULT_TYPE", "postgresql");
        std::env::set_var("V_DATABASE_DEFAULT_HOST", "127.0.0.1");
        std::env::set_var("V_DATABASE_DEFAULT_PORT", "5432");
        std::env::set_var("V_DATABASE_DEFAULT_USER", "postgres");
        std::env::set_var("V_DATABASE_DEFAULT_PASS", "");
        std::env::set_var("V_DATABASE_DEFAULT_NAME", "postgres");
        std::env::set_var("V_DATABASE_DEFAULT_MAXOPEN", "2");

        DatabaseManager::reset_postgres_pools().await;

        // 改用通用 AnyPool 接口
        let p1 = DatabaseManager::get_any_pool_by_group("default").await.unwrap();
        let p2 = DatabaseManager::get_any_pool_by_group("default").await.unwrap();
        assert_eq!(DatabaseManager::pool_count_any().await, 1);

        // 进行健康检查（若本地未运行数据库，可能失败，但不会 panic）
        let _ = DatabaseManager::check_any_health(&p1).await;
        let _ = DatabaseManager::check_any_health(&p2).await;
    }

    #[tokio::test]
    async fn test_any_pool_generic_init() {
        // 配置 default 组为 sqlite，使用内存库
        std::env::set_var("V_DATABASE_DEFAULT_TYPE", "sqlite");
        std::env::set_var("V_DATABASE_DEFAULT_URL", "sqlite::memory:");
        std::env::set_var("V_DATABASE_DEFAULT_MAXOPEN", "1");

        DatabaseManager::reset_any_pools().await;

        // 使用模型组名：这里直接用 Base 模型名占位（无需存在）；测试通过 group 访问
        struct Dummy;
        impl super::Model for Dummy {
            fn table_name() -> &'static str { "dummy" }
            fn group_name() -> &'static str { "default" }
        }
        let p = DatabaseManager::get_db_pool::<Dummy>().await.unwrap();
        assert_eq!(DatabaseManager::pool_count_any().await, 1);
        let _ = DatabaseManager::check_any_health(&p).await;
    }
}