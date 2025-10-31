use actix_web::web;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::RwLock;

/// 路由配置函数类型
pub type RouteConfigFn = fn(&mut web::ServiceConfig);

/// 路由信息结构
#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub name: String,
    pub description: String,
    pub module: String,
    pub config_fn: RouteConfigFn,
}

/// 全局路由注册器
#[derive(Debug)]
#[allow(dead_code)]
pub struct RouteRegistry {
    routes: HashMap<String, RouteInfo>,
}

#[allow(dead_code)]
impl RouteRegistry {
    /// 创建新的路由注册器
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    /// 注册路由
    pub fn register_route(&mut self, route_info: RouteInfo) {
        self.routes.insert(route_info.name.clone(), route_info);
    }

    /// 获取所有路由
    pub fn get_routes(&self) -> &HashMap<String, RouteInfo> {
        &self.routes
    }

    /// 获取指定模块的路由
    pub fn get_routes_by_module(&self, module: &str) -> Vec<&RouteInfo> {
        self.routes
            .values()
            .filter(|route| route.module == module)
            .collect()
    }

    /// 配置所有路由到 ServiceConfig
    pub fn configure_all_routes(&self, cfg: &mut web::ServiceConfig) {
        for route_info in self.routes.values() {
            (route_info.config_fn)(cfg);
        }
    }

    /// 配置指定模块的路由
    pub fn configure_module_routes(&self, cfg: &mut web::ServiceConfig, module: &str) {
        for route_info in self.get_routes_by_module(module) {
            (route_info.config_fn)(cfg);
        }
    }

    /// 获取路由统计信息
    pub fn get_stats(&self) -> (usize, Vec<String>) {
        let total = self.routes.len();
        let modules: Vec<String> = self
            .routes
            .values()
            .map(|route| route.module.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        (total, modules)
    }

    /// 打印路由信息
    pub fn print_routes_info(&self) {
        println!("路由注册信息:");
        println!("============");

        let mut modules: Vec<String> = self
            .routes
            .values()
            .map(|route| route.module.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        modules.sort();

        for module in modules {
            let module_routes = self.get_routes_by_module(&module);
            println!("模块: {} ({} 个路由)", module, module_routes.len());
            for route in module_routes {
                println!("  - {}: {}", route.name, route.description);
            }
        }

        let (total, _) = self.get_stats();
        println!("总计: {} 个路由", total);
    }
}

impl Default for RouteRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// 全局路由注册器实例
lazy_static! {
    static ref GLOBAL_ROUTE_REGISTRY: RwLock<RouteRegistry> = RwLock::new(RouteRegistry::new());
}

/// 获取全局路由注册器的读锁
#[allow(dead_code)]
pub fn get_global_route_registry() -> std::sync::RwLockReadGuard<'static, RouteRegistry> {
    GLOBAL_ROUTE_REGISTRY.read().unwrap()
}

/// 获取全局路由注册器的写锁
#[allow(dead_code)]
pub fn get_global_route_registry_mut() -> std::sync::RwLockWriteGuard<'static, RouteRegistry> {
    GLOBAL_ROUTE_REGISTRY.write().unwrap()
}

/// 注册路由到全局注册器
#[allow(dead_code)]
pub fn register_global_route(route_info: RouteInfo) {
    let mut registry = get_global_route_registry_mut();
    registry.register_route(route_info);
}

/// 配置所有全局路由
#[allow(dead_code)]
pub fn configure_global_routes(cfg: &mut web::ServiceConfig) {
    let registry = get_global_route_registry();
    registry.configure_all_routes(cfg);
}

/// 配置指定模块的全局路由
#[allow(dead_code)]
pub fn configure_global_module_routes(cfg: &mut web::ServiceConfig, module: &str) {
    let registry = get_global_route_registry();
    registry.configure_module_routes(cfg, module);
}

/// 打印全局路由信息
#[allow(dead_code)]
pub fn print_global_routes_info() {
    let registry = get_global_route_registry();
    registry.print_routes_info();
}

/// 获取全局路由统计信息
#[allow(dead_code)]
pub fn get_global_routes_stats() -> (usize, Vec<String>) {
    let registry = get_global_route_registry();
    registry.get_stats()
}

/// 便捷宏：注册路由
#[macro_export]
macro_rules! register_route {
    ($name:expr, $description:expr, $module:expr, $config_fn:expr) => {
        $crate::bootstrap::route_registry::register_global_route(
            $crate::bootstrap::route_registry::RouteInfo {
                name: $name.to_string(),
                description: $description.to_string(),
                module: $module.to_string(),
                config_fn: $config_fn,
            },
        );
    };
}

/// 便捷宏：批量注册路由
#[macro_export]
macro_rules! register_routes {
    ($(($name:expr, $description:expr, $module:expr, $config_fn:expr)),* $(,)?) => {
        $(
            register_route!($name, $description, $module, $config_fn);
        )*
    };
}
