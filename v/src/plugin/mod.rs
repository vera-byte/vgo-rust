use anyhow::Result;
pub use serde_json::{json, Value};

/// 通用插件接口 / Generic plugin interface
pub trait VPlugin {
    /// 创建实例 / Create instance
    fn new(config: Option<Value>) -> Self
    where
        Self: Sized;

    /// 运行插件（阻塞或异步封装）/ Run plugin (blocking or async wrapper)
    fn run(&mut self) -> Result<()>;

    /// 应用配置 / Apply configuration
    fn config(&mut self, cfg: &Value) -> Result<()>;
}

pub mod client;
pub mod events;
pub mod installer;
pub mod pdk;
pub mod types;
