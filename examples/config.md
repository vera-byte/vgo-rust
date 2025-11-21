
**核心区别**
- `manager.get(...)` 是实例方法：你自己持有一个 `ConfigManager`（通常是 `Arc<ConfigManager>`），直接在该实例上读取配置。
- `v::get_config(...)` 是全局便捷函数：内部会先获取（或初始化）全局单例 `ConfigManager`，再调用它的 `get(...)`。

**行为与范围**
- `manager.get`：
  - 作用在“你指定的管理器实例”上，可用默认源或自定义源（通过 `ConfigManager::with_sources(...)`）。
  - 适合依赖注入、单元测试和特殊场景（比如临时加载某个字符串/内存源）。
- `v::get_config`：
  - 作用在“全局单例管理器”上，使用库内预设的源与优先级：
    - `config/development.toml` -> `config/default.toml` -> `config/production.toml` -> 环境变量（前缀 `V`，分隔 `_`）
  - 适合简单场景或全局配置读取，不需要显式管理生命周期。

**错误类型与开销**
- 两者返回类型一致：`thiserror::Error<T>`。
- `v::get_config` 可能额外失败在“初始化全局管理器”阶段（例如必要文件缺失）；`manager.get` 没有这一步。
- 并发与性能：
  - `v::get_config` 每次会通过 `RwLock` 读取全局管理器（首次可能写锁初始化），随后开销很小，但仍有锁。
  - 持有并复用一个 `Arc<ConfigManager>` 调用 `manager.get`，可避免每次获取全局锁，适合高频热路径。

**进阶能力**
- 仅 `manager` 可访问更丰富方法：`get_safe`（返回细化的 `ConfigError`）、`print_sources_info`、源统计等。
- 全局函数还有其他便捷函数：
  - `v::get_config_safe`：包装了全局初始化错误为 `ConfigError::InitializationError` 后再做类型化错误。
  - `v::get_config_cached_simple`：简单缓存基础类型的读取结果（使用 `CONFIG_CACHE`）。

**什么时候用哪个**
- 用 `v::get_config`：
  - 快速读取、无需自定义源。
  - CLI、演示示例、应用启动阶段的少量读取。
- 用 `manager.get`：
  - 性能敏感的热路径（避免锁），在异步任务中传递 `Arc<ConfigManager>`。
  - 需要自定义加载源或隔离测试环境。
  - 需要访问更细粒度的诊断接口（源信息、统计、类型化错误）。

**示例对比**
- 使用全局读取：
  - `let host: String = v::get_config("server.host")?;`
- 复用实例避免锁并自定义源：
  - `let mgr = v::get_global_config_manager()?;`
  - `let host: String = mgr.get("server.host")?;`
  - 或者
  - `let mgr = ConfigManager::with_sources(vec![/* 自定义 */])?;`
  - `let host: String = mgr.get("server.host")?;`

总结：`v::get_config` 更简洁、统一全局行为；`manager.get` 更灵活、可注入且在高频场景更高效。根据你的使用场景选择即可。
        