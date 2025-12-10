//! 插件运行时管理模块 / Plugin runtime management module
//!
//! 负责插件的加载、启动、停止和通信
//! Responsible for plugin loading, starting, stopping and communication

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use futures_util::future;
use parking_lot::RwLock;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::{Child, Command};
use tokio::sync::watch;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use v::plugin::installer::PluginInstaller;
use prost::Message; // For Protobuf decoding

/// 插件状态 / Plugin status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginStatus {
    /// 已安装但未启动 / Installed but not started
    Installed,
    /// 启动中 / Starting
    Starting,
    /// 运行中 / Running
    Running,
    /// 停止中 / Stopping
    Stopping,
    /// 已停止 / Stopped
    Stopped,
    /// 错误状态 / Error state
    Error(String),
}

/// 插件运行时信息 / Plugin runtime information
pub struct PluginRuntime {
    pub name: String,
    pub path: PathBuf,
    pub version: Option<String>,
    pub status: Arc<RwLock<PluginStatus>>,
    pub process: Arc<RwLock<Option<Child>>>, // 进程句柄 / Process handle
    pub socket_path: Option<PathBuf>,
    pub last_heartbeat: Arc<RwLock<Option<Instant>>>,
    pub capabilities: Arc<RwLock<Vec<String>>>, // 插件能力 / Plugin capabilities
    pub priority: Arc<RwLock<i32>>,             // 插件优先级 / Plugin priority
}

impl PluginRuntime {
    pub fn new(
        name: String,
        path: PathBuf,
        version: Option<String>,
        socket_path: Option<PathBuf>,
    ) -> Self {
        Self {
            name,
            path,
            version,
            status: Arc::new(RwLock::new(PluginStatus::Installed)),
            process: Arc::new(RwLock::new(None)),
            socket_path,
            last_heartbeat: Arc::new(RwLock::new(None)),
            capabilities: Arc::new(RwLock::new(Vec::new())),
            priority: Arc::new(RwLock::new(0)),
        }
    }

    /// 设置能力 / Set capabilities
    pub fn set_capabilities(&self, caps: Vec<String>) {
        *self.capabilities.write() = caps;
    }

    /// 获取能力 / Get capabilities
    pub fn capabilities(&self) -> Vec<String> {
        self.capabilities.read().clone()
    }

    /// 设置优先级 / Set priority
    pub fn set_priority(&self, p: i32) {
        *self.priority.write() = p;
    }

    /// 获取优先级 / Get priority
    pub fn priority(&self) -> i32 {
        *self.priority.read()
    }

    /// 获取状态 / Get status
    pub fn status(&self) -> PluginStatus {
        self.status.read().clone()
    }

    /// 设置状态 / Set status
    pub fn set_status(&self, status: PluginStatus) {
        *self.status.write() = status;
    }
}

/// 插件运行时管理器 / Plugin runtime manager
pub struct PluginRuntimeManager {
    plugins: DashMap<String, PluginRuntime>,
    plugin_dir: PathBuf,
    socket_dir: PathBuf,
    global_socket_path: Option<PathBuf>,
    debug_mode: bool,          // Debug 模式 / Debug mode
    log_level: Option<String>, // 日志级别 / Log level
}

/// 插件元数据 / Plugin metadata
#[derive(Clone, Default)]
struct PluginMetadata {
    plugin_no: Option<String>,
    version: Option<String>,
}

/// 运行时插件摘要 / Runtime plugin summary info
#[derive(Clone)]
pub struct PluginRuntimeSummary {
    pub name: String,
    pub version: Option<String>,
    pub status: PluginStatus,
}

impl PluginRuntimeManager {
    /// 创建新的插件运行时管理器 / Create new plugin runtime manager
    pub fn new(plugin_dir: impl AsRef<Path>, socket_dir: impl AsRef<Path>) -> Self {
        Self {
            plugins: DashMap::new(),
            plugin_dir: plugin_dir.as_ref().to_path_buf(),
            socket_dir: socket_dir.as_ref().to_path_buf(),
            global_socket_path: None,
            debug_mode: false,
            log_level: None,
        }
    }

    /// 设置 debug 模式 / Set debug mode
    pub fn set_debug_mode(&mut self, debug: bool) {
        self.debug_mode = debug;
    }

    /// 设置日志级别 / Set log level
    pub fn set_log_level(&mut self, level: String) {
        self.log_level = Some(level);
    }

    /// 设置全局 socket 路径（所有插件共享）/ Set global socket path shared by all plugins
    pub fn set_global_socket_path(&mut self, path: impl AsRef<Path>) {
        self.global_socket_path = Some(path.as_ref().to_path_buf());
    }

    /// 注册开发模式插件 / Register development mode plugin
    pub fn register_dev_plugin(&self, name: String, cargo_project_path: PathBuf) -> Result<()> {
        info!(
            "🛠️ Registering dev plugin: {} from {}",
            name,
            cargo_project_path.display()
        );

        let socket_path = self.global_socket_path.clone();
        let runtime = PluginRuntime::new(
            name.clone(),
            cargo_project_path,
            Some("dev".to_string()),
            socket_path,
        );

        self.plugins.insert(name, runtime);
        Ok(())
    }

    /// 初始化运行时管理器 / Initialize runtime manager
    pub fn init(&self) -> Result<()> {
        // 创建 socket 目录 / Create socket directory
        if !self.socket_dir.exists() {
            std::fs::create_dir_all(&self.socket_dir)?;
            info!("Created plugin socket directory: {:?}", self.socket_dir);
        }
        Ok(())
    }

    /// 发现并加载已安装的插件 / Discover and load installed plugins
    pub async fn discover_plugins(&self) -> Result<Vec<String>> {
        let installer = PluginInstaller::new(&self.plugin_dir);
        let installed = installer.list_installed()?;

        info!("Discovered {} installed plugin(s)", installed.len());
        for name in &installed {
            debug!("Found plugin: {}", name);
        }

        Ok(installed)
    }

    /// 启动插件 / Start plugin
    pub async fn start_plugin(&self, name: &str) -> Result<()> {
        info!("🚀 正在启动插件 / Starting plugin: {}", name);

        // 检查是否已存在 / Check if already exists
        if let Some(runtime) = self.plugins.get(name) {
            let status = runtime.status();
            if matches!(status, PluginStatus::Running | PluginStatus::Starting) {
                warn!("Plugin {} is already running", name);
                return Ok(());
            }
        }

        // 查找插件二进制文件 / Find plugin binary
        debug!("查找插件二进制文件 / Looking for plugin binary: {}", name);
        let plugin_path = self.find_plugin_binary(name)?;
        info!("✅ 找到插件二进制 / Found plugin binary: {:?}", plugin_path);
        let socket_path = if let Some(global) = &self.global_socket_path {
            global.clone()
        } else {
            let path = self.socket_dir.join(format!("{}.sock", name));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            path
        };
        let owned_socket = if self.global_socket_path.is_some() {
            None
        } else {
            Some(socket_path.clone())
        };

        let metadata = self.read_plugin_metadata(name);
        let runtime = PluginRuntime::new(
            name.to_string(),
            plugin_path,
            metadata.version.clone(),
            owned_socket.clone(),
        );
        runtime.set_status(PluginStatus::Starting);

        // 启动插件进程 / Start plugin process
        let mut cmd = if runtime.path.is_dir() {
            // 开发模式：使用 cargo run / Dev mode: use cargo run
            info!("🛠️ Starting dev plugin {} with cargo run", name);
            let mut c = Command::new("cargo");
            c.arg("run")
                .arg("--manifest-path")
                .arg(runtime.path.join("Cargo.toml"))
                .arg("--")
                .current_dir(&runtime.path);
            c
        } else {
            // 生产模式：直接运行二进制 / Production mode: run binary directly
            Command::new(&runtime.path)
        };

        // 创建插件日志目录 / Create plugin log directory
        let log_dir = PathBuf::from("./logs/plugins").join(name);
        if let Err(e) = std::fs::create_dir_all(&log_dir) {
            warn!("Failed to create plugin log directory {:?}: {}", log_dir, e);
        }

        // 创建日志文件 / Create log files
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let stdout_log = log_dir.join(format!("stdout_{}.log", timestamp));
        let stderr_log = log_dir.join(format!("stderr_{}.log", timestamp));

        let stdout_file = match std::fs::File::create(&stdout_log) {
            Ok(f) => {
                info!("📝 Plugin {} stdout log: {:?}", name, stdout_log);
                Stdio::from(f)
            }
            Err(e) => {
                warn!(
                    "Failed to create stdout log file {:?}: {}, using inherit",
                    stdout_log, e
                );
                Stdio::inherit()
            }
        };

        let stderr_file = match std::fs::File::create(&stderr_log) {
            Ok(f) => {
                info!("📝 Plugin {} stderr log: {:?}", name, stderr_log);
                Stdio::from(f)
            }
            Err(e) => {
                warn!(
                    "Failed to create stderr log file {:?}: {}, using inherit",
                    stderr_log, e
                );
                Stdio::inherit()
            }
        };

        cmd.arg("--socket")
            .arg(socket_path.to_string_lossy().as_ref())
            .stdin(Stdio::null())
            .stdout(stdout_file)
            .stderr(stderr_file);

        // 添加 debug 参数 / Add debug arguments
        if self.debug_mode {
            cmd.arg("--debug");
            info!("Starting plugin {} in debug mode", name);
        }

        // 添加日志级别参数 / Add log level argument
        if let Some(ref level) = self.log_level {
            cmd.arg("--log-level").arg(level);
            info!("Starting plugin {} with log level: {}", name, level);
        }

        match cmd.spawn() {
            Ok(child) => {
                let child_arc = Arc::new(RwLock::new(Some(child)));
                // 存储进程引用 / Store process reference (实际句柄在 child_arc 中)
                *runtime.process.write() = None;

                runtime.set_status(PluginStatus::Running);

                // 监控插件进程 / Monitor plugin process
                let name_clone = name.to_string();
                let status_clone = runtime.status.clone();
                let last_heartbeat_clone = runtime.last_heartbeat.clone();
                let process_clone = runtime.process.clone();
                tokio::spawn(async move {
                    // 将 child 移动到 process 中 / Move child to process
                    if let Some(child) = child_arc.write().take() {
                        *process_clone.write() = Some(child);
                    }
                    Self::monitor_plugin_process(
                        name_clone,
                        process_clone,
                        status_clone,
                        last_heartbeat_clone,
                    )
                    .await;
                });

                self.plugins.insert(name.to_string(), runtime);
                info!("Plugin {} started", name);
                Ok(())
            }
            Err(e) => {
                runtime.set_status(PluginStatus::Error(e.to_string()));
                Err(anyhow!("Failed to start plugin {}: {}", name, e))
            }
        }
    }

    /// 停止插件 / Stop plugin
    pub async fn stop_plugin(&self, name: &str) -> Result<()> {
        if let Some(runtime) = self.plugins.get(name) {
            info!("🛑 正在停止插件 / Stopping plugin: {}", name);
            runtime.set_status(PluginStatus::Stopping);

            // 终止进程 / Terminate process
            let mut killed = false;
            if let Some(mut child) = {
                let mut guard = runtime.process.write();
                guard.take()
            } {
                // 先尝试优雅终止 / Try graceful termination first
                if let Err(e) = child.kill().await {
                    error!("Failed to kill plugin {}: {}", name, e);
                } else {
                    // 等待进程退出，最多等待 3 秒 / Wait for process exit, max 3 seconds
                    match tokio::time::timeout(Duration::from_secs(3), child.wait()).await {
                        Ok(Ok(status)) => {
                            info!(
                                "✅ 插件 {} 已退出 / Plugin {} exited with status: {:?}",
                                name, name, status
                            );
                            killed = true;
                        }
                        Ok(Err(e)) => {
                            error!(
                                "❌ 等待插件 {} 退出失败 / Failed to wait plugin {} exit: {}",
                                name, name, e
                            );
                        }
                        Err(_) => {
                            warn!("⏰ 插件 {} 退出超时 / Plugin {} exit timeout", name, name);
                        }
                    }
                }
            } else {
                debug!("插件 {} 进程句柄不存在，尝试通过名称杀死进程 / Plugin {} process handle not found, trying to kill by name", name, name);
            }

            // 如果进程句柄不存在或杀死失败，尝试使用 pkill / If process handle not found or kill failed, try pkill
            if !killed {
                #[cfg(unix)]
                {
                    // 先检查是否有相关进程在运行 / First check if there are related processes running
                    let pgrep_result = tokio::process::Command::new("pgrep")
                        .arg("-f")
                        .arg(name)
                        .output()
                        .await;

                    match pgrep_result {
                        Ok(output) => {
                            if output.status.success() && !output.stdout.is_empty() {
                                let pids = String::from_utf8_lossy(&output.stdout);
                                info!("🔍 找到插件 {} 的进程 PID: {} / Found plugin {} processes with PIDs: {}", 
                                      name, pids.trim(), name, pids.trim());

                                // 尝试使用 pkill 杀死插件进程 / Try to kill plugin process using pkill
                                let pkill_result = tokio::process::Command::new("pkill")
                                    .arg("-9") // 使用 SIGKILL 强制终止 / Use SIGKILL to force terminate
                                    .arg("-f")
                                    .arg(name)
                                    .output()
                                    .await;

                                match pkill_result {
                                    Ok(output) => {
                                        if output.status.success() {
                                            info!("✅ 使用 pkill 成功终止插件 {} / Successfully killed plugin {} using pkill", name, name);
                                            // 等待一小段时间让进程真正退出 / Wait a moment for process to actually exit
                                            debug!("⏳ 等待 500ms 让进程退出 / Waiting 500ms for process to exit");
                                            tokio::time::sleep(Duration::from_millis(500)).await;
                                            debug!("✅ 等待完成 / Wait completed");
                                        } else {
                                            warn!("⚠️  pkill 执行失败 / pkill execution failed");
                                        }
                                    }
                                    Err(e) => {
                                        error!("❌ pkill 执行错误 / pkill execution error: {}", e);
                                    }
                                }
                            } else {
                                debug!("✅ 未找到插件 {} 的运行进程（可能已退出）/ No running process found for plugin {} (may have already exited)", name, name);
                            }
                        }
                        Err(e) => {
                            debug!("pgrep 执行失败 / pgrep execution failed: {}", e);
                        }
                    }
                }
            }

            // 清理 socket / Cleanup socket
            debug!("🧹 开始清理 socket / Starting socket cleanup");
            if let Some(socket_path) = &runtime.socket_path {
                if let Err(e) = std::fs::remove_file(socket_path) {
                    debug!("清理 socket 文件失败 / Failed to remove socket file: {}", e);
                }
            }
            debug!("✅ Socket 清理完成 / Socket cleanup completed");

            debug!("📝 更新插件状态 / Updating plugin status");
            runtime.set_status(PluginStatus::Stopped);

            // 必须先释放 runtime 引用，否则 remove 会死锁 / Must drop runtime reference first, otherwise remove will deadlock
            debug!("🔓 释放插件引用 / Dropping plugin reference");
            drop(runtime);

            debug!("🗑️  从插件列表移除 / Removing from plugin list");
            let before_size = self.plugins.len();
            debug!("🔍 插件列表当前大小: {}", before_size);
            self.plugins.remove(name);
            let after_size = self.plugins.len();
            debug!("✅ 插件已从列表移除 / Plugin removed from list");
            debug!("🔍 插件列表移除后大小: {}", after_size);
            info!("✅ 插件 {} 已停止 / Plugin {} stopped", name, name);
            debug!("🎯 stop_plugin 方法即将返回 / stop_plugin method about to return");
            Ok(())
        } else {
            warn!("插件 {} 未找到 / Plugin {} not found", name, name);
            Ok(()) // 不返回错误，避免阻塞其他插件的停止 / Don't return error to avoid blocking other plugins
        }
    }

    /// 查找插件二进制文件 / Find plugin binary
    fn find_plugin_binary(&self, name: &str) -> Result<PathBuf> {
        let plugin_dir = self.plugin_dir.join(name);

        if !plugin_dir.exists() {
            return Err(anyhow!("Plugin directory not found: {:?}", plugin_dir));
        }

        // 查找可执行文件 / Find executable
        let exe_name = if cfg!(target_os = "windows") {
            format!("{}.exe", name)
        } else {
            name.to_string()
        };

        let exe_path = plugin_dir.join(&exe_name);
        if exe_path.exists() && exe_path.is_file() {
            return Ok(exe_path);
        }

        // 尝试查找其他可能的二进制文件 / Try to find other possible binaries
        let entries = std::fs::read_dir(&plugin_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                // 检查是否有执行权限（Unix）或是否为 .exe（Windows）
                // Check if executable (Unix) or .exe (Windows)
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = path.metadata() {
                        let perms = metadata.permissions();
                        if perms.mode() & 0o111 != 0 {
                            return Ok(path);
                        }
                    }
                }
                #[cfg(windows)]
                {
                    if path.extension().and_then(|s| s.to_str()) == Some("exe") {
                        return Ok(path);
                    }
                }
            }
        }

        Err(anyhow!("Plugin binary not found in {:?}", plugin_dir))
    }

    /// 监控插件进程 / Monitor plugin process
    async fn monitor_plugin_process(
        name: String,
        process: Arc<RwLock<Option<Child>>>,
        status: Arc<RwLock<PluginStatus>>,
        last_heartbeat: Arc<RwLock<Option<Instant>>>,
    ) {
        loop {
            sleep(Duration::from_secs(1)).await;

            let mut process_guard = process.write();
            if let Some(child) = process_guard.as_mut() {
                match child.try_wait() {
                    Ok(Some(exit_status)) => {
                        if exit_status.success() {
                            info!("Plugin {} exited successfully", name);
                        } else {
                            error!("Plugin {} exited with error: {:?}", name, exit_status);
                            *status.write() =
                                PluginStatus::Error(format!("Process exited: {:?}", exit_status));
                        }
                        *process_guard = None;
                        break;
                    }
                    Ok(None) => {
                        // 进程仍在运行 / Process still running
                        *last_heartbeat.write() = Some(Instant::now());
                    }
                    Err(e) => {
                        error!("Error checking plugin {} status: {}", name, e);
                        *status.write() = PluginStatus::Error(e.to_string());
                        *process_guard = None;
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    /// 启动所有已安装的插件 / Start all installed plugins
    pub async fn start_all(&self) -> Result<()> {
        let installed = self.discover_plugins().await?;

        for name in installed {
            if let Err(e) = self.start_plugin(&name).await {
                error!("Failed to start plugin {}: {}", name, e);
            }
        }

        Ok(())
    }

    /// 停止所有插件 / Stop all plugins
    pub async fn stop_all(&self) -> Result<()> {
        let names: Vec<String> = self.plugins.iter().map(|e| e.key().clone()).collect();

        if names.is_empty() {
            info!("没有需要停止的插件 / No plugins to stop");
            return Ok(());
        }

        info!(
            "🛑 正在停止 {} 个插件 / Stopping {} plugins",
            names.len(),
            names.len()
        );

        // 并发停止所有插件，最多等待 5 秒 / Stop all plugins concurrently, max 5 seconds
        debug!("📦 创建停止任务 / Creating stop tasks");
        let stop_futures: Vec<_> = names.iter().map(|name| self.stop_plugin(name)).collect();

        debug!("⏳ 等待所有插件停止（最多5秒）/ Waiting for all plugins to stop (max 5s)");
        match tokio::time::timeout(Duration::from_secs(5), future::join_all(stop_futures)).await {
            Ok(results) => {
                debug!("✅ 所有插件停止任务完成 / All plugin stop tasks completed");
                let mut success_count = 0;
                let mut error_count = 0;
                for (name, result) in names.iter().zip(results) {
                    match result {
                        Ok(_) => success_count += 1,
                        Err(e) => {
                            error!(
                                "❌ 停止插件 {} 失败 / Failed to stop plugin {}: {}",
                                name, name, e
                            );
                            error_count += 1;
                        }
                    }
                }
                info!("✅ 插件停止完成：成功 {} 个，失败 {} 个 / Plugin stop completed: {} succeeded, {} failed", 
                      success_count, error_count, success_count, error_count);
            }
            Err(_) => {
                warn!("⏰ 停止插件超时（5秒），继续关闭 / Stop plugins timeout (5s), continuing shutdown");
            }
        }

        Ok(())
    }

    /// 获取运行时摘要 / Collect runtime summaries
    pub fn runtime_summaries(&self) -> Vec<PluginRuntimeSummary> {
        self.plugins
            .iter()
            .map(|entry| {
                let runtime = entry.value();
                PluginRuntimeSummary {
                    name: runtime.name.clone(),
                    version: runtime.version.clone(),
                    status: runtime.status(),
                }
            })
            .collect()
    }

    fn read_plugin_metadata(&self, name: &str) -> PluginMetadata {
        let manifest = self.plugin_dir.join(name).join("plugin.json");
        if let Ok(content) = std::fs::read_to_string(&manifest) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                let plugin_no = value
                    .get("plugin_no")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let version = value
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                return PluginMetadata { plugin_no, version };
            }
        }
        PluginMetadata::default()
    }
}

/// Unix Socket 服务器 / Unix Socket server
pub struct UnixSocketServer {
    listener: UnixListener,
    plugin_manager: Arc<PluginRuntimeManager>,
    connection_pool: Arc<PluginConnectionPool>,
    shutdown_rx: watch::Receiver<bool>, // 关闭信号 / Shutdown signal
}

impl UnixSocketServer {
    /// 创建并启动 Unix Socket 服务器 / Create and start Unix Socket server
    pub async fn new(
        socket_path: impl AsRef<Path>,
        plugin_manager: Arc<PluginRuntimeManager>,
        shutdown_rx: watch::Receiver<bool>,
    ) -> Result<Self> {
        // 删除已存在的 socket / Remove existing socket
        let socket_path = socket_path.as_ref();
        if let Some(parent) = socket_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        if socket_path.exists() {
            std::fs::remove_file(socket_path)?;
        }

        let listener = UnixListener::bind(socket_path)?;
        info!("Unix Socket server listening on: {:?}", socket_path);

        let connection_pool = Arc::new(PluginConnectionPool::new(plugin_manager.clone()));

        Ok(Self {
            listener,
            plugin_manager,
            connection_pool,
            shutdown_rx,
        })
    }

    /// 获取连接池 / Get connection pool
    pub fn connection_pool(&self) -> Arc<PluginConnectionPool> {
        self.connection_pool.clone()
    }

    /// 运行服务器 / Run server
    pub async fn run(&self) -> Result<()> {
        let mut rx = self.shutdown_rx.clone();
        loop {
            tokio::select! {
                res = self.listener.accept() => {
                    match res {
                        Ok((stream, _)) => {
                            let manager = self.plugin_manager.clone();
                            let pool = self.connection_pool.clone();
                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(stream, manager, pool).await {
                                    error!("Error handling Unix Socket connection: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            // 接受连接错误（可能在关闭期间出现）/ Accept error (may occur during shutdown)
                            error!("Error accepting Unix Socket connection: {}", e);
                        }
                    }
                }
                _ = rx.changed() => {
                    if *rx.borrow() {
                        info!("🛑 Unix Socket server shutdown signal received");
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    /// 处理连接 / Handle connection
    async fn handle_connection(
        stream: UnixStream,
        manager: Arc<PluginRuntimeManager>,
        pool: Arc<PluginConnectionPool>,
    ) -> Result<()> {
        let (mut read_half, mut write_half) = stream.into_split();
        let mut plugin_name: Option<String> = None;
        let mut handshake_done = false;

        loop {
            match read_half.read_u32().await {
                Ok(len) => {
                    let mut buffer = vec![0u8; len as usize];
                    if let Err(e) = read_half.read_exact(&mut buffer).await {
                        error!("Plugin connection read error: {}", e);
                        break;
                    }

                    // 尝试解析握手消息（支持 Protobuf 和 JSON）
                    // Try to parse handshake message (support both Protobuf and JSON)
                    if !handshake_done {
                        // 处理握手 / Handle handshake
                        handshake_done = true;

                        let (name, version, capabilities, priority) = 
                            // 先尝试 Protobuf 格式 / Try Protobuf first
                            if let Ok(handshake) = v::plugin::protocol::HandshakeRequest::decode(&buffer[..]) {
                                (
                                    handshake.name,
                                    handshake.version,
                                    handshake.capabilities,
                                    handshake.priority,
                                )
                            } else {
                                // 回退到 JSON 格式（向后兼容）/ Fallback to JSON (backward compatible)
                                let payload: Value = serde_json::from_slice(&buffer).unwrap_or(Value::Null);
                                let name = payload
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                let version = payload
                                    .get("version")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                let capabilities = payload
                                    .get("capabilities")
                                    .and_then(|v| v.as_array())
                                    .map(|arr| {
                                        arr.iter()
                                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default();
                                let priority = payload
                                    .get("priority")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0) as i32;
                                (name, version, capabilities, priority)
                            };

                        plugin_name = Some(name.clone());

                        info!(
                            "🤝 Plugin handshake: {} v{} (priority: {}, capabilities: {:?})",
                            name, version, priority, capabilities
                        );

                        // 保存插件信息 / Save plugin info
                        // name 是插件的 PLUGIN_NO (例如 "v.plugin.storage-sled")
                        // 需要找到对应的运行时插件（目录名，例如 "v-connect-im-plugin-storage-sled"）
                        let mut found = false;
                        let mut matched_key: Option<String> = None;

                        // 遍历所有已注册的插件，通过 plugin.json 中的 plugin_no 匹配
                        for entry in manager.plugins.iter() {
                            let key = entry.key();
                            let metadata = manager.read_plugin_metadata(key);
                            if let Some(plugin_no) = metadata.plugin_no {
                                if plugin_no == name {
                                    matched_key = Some(key.clone());
                                    break;
                                }
                            }

                            // 如果没有 plugin_no，尝试通过名称匹配
                            if matched_key.is_none() {
                                let short_name = name
                                    .strip_prefix("v.plugin.")
                                    .unwrap_or(&name);

                                if key == &name
                                    || key == short_name
                                    || key.contains(short_name)
                                    || key.ends_with(short_name)
                                {
                                    matched_key = Some(key.clone());
                                    break;
                                }
                            }
                        }

                        if let Some(ref key) = matched_key {
                            if let Some(runtime) = manager.plugins.get(key) {
                                runtime.set_capabilities(capabilities.clone());
                                runtime.set_priority(priority);
                                runtime.set_status(PluginStatus::Running);
                                found = true;
                                info!(
                                    "✅ 插件信息已更新 / Plugin info updated: {} -> {}",
                                    name, key
                                );
                            }
                        }

                        // 确定注册名称：使用匹配到的运行时名称 / Determine registration name
                        let register_name = matched_key.unwrap_or_else(|| name.to_string());

                        if !found {
                            warn!(
                                "⚠️  未找到插件运行时信息 / Plugin runtime not found: {}",
                                name
                            );
                            debug!(
                                "已注册的插件列表 / Registered plugins: {:?}",
                                manager
                                    .plugins
                                    .iter()
                                    .map(|e| e.key().clone())
                                    .collect::<Vec<_>>()
                            );
                        }

                        // 发送握手响应 / Send handshake response
                        let handshake_response = v::plugin::protocol::HandshakeResponse {
                            status: "ok".to_string(),
                            message: "Handshake successful".to_string(),
                            config: String::new(), // 配置通过单独的 config 消息发送
                            protocol: "protobuf".to_string(),
                        };
                        let response = handshake_response.encode_to_vec();
                        write_half.write_u32(response.len() as u32).await?;
                        write_half.write_all(&response).await?;
                        write_half.flush().await?;

                        // 重新组合 stream 并注册到连接池 / Reunite stream and register to pool
                        let reunited = read_half.reunite(write_half)?;
                        pool.register(register_name.clone(), reunited);

                        info!(
                            "✅ Plugin {} registered to connection pool as '{}'",
                            name, register_name
                        );
                        return Ok(());
                    }
                }
                Err(e) => {
                    // 连接关闭（EOF常见于优雅停机）/ Connection closed (EOF common on graceful shutdown)
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        info!(
                            "Plugin {} connection closed gracefully (EOF)",
                            plugin_name.as_deref().unwrap_or("unknown")
                        );
                    } else {
                        debug!(
                            "Plugin {} connection closed: {}",
                            plugin_name.as_deref().unwrap_or("unknown"),
                            e
                        );
                    }

                    // 从连接池移除 / Remove from connection pool
                    if let Some(name) = &plugin_name {
                        pool.unregister(name);
                    }
                    break;
                }
            }
        }

        drop(manager);
        Ok(())
    }
}

/// 插件连接池 / Plugin connection pool
pub struct PluginConnectionPool {
    connections: Arc<DashMap<String, Arc<tokio::sync::Mutex<UnixStream>>>>,
    manager: Arc<PluginRuntimeManager>,
}

impl PluginConnectionPool {
    pub fn new(manager: Arc<PluginRuntimeManager>) -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            manager,
        }
    }

    /// 注册插件连接 / Register plugin connection
    pub fn register(&self, name: String, stream: UnixStream) {
        self.connections
            .insert(name, Arc::new(tokio::sync::Mutex::new(stream)));
    }

    /// 移除插件连接 / Remove plugin connection
    pub fn unregister(&self, name: &str) {
        self.connections.remove(name);
    }

    /// 关闭所有插件连接 / Close all plugin connections
    pub async fn close_all(&self) {
        let count = self.connections.len();
        if count > 0 {
            info!(
                "🔌 关闭 {} 个插件连接 / Closing {} plugin connections",
                count, count
            );

            // 显式关闭每个连接 / Explicitly close each connection
            let names: Vec<String> = self.connections.iter().map(|e| e.key().clone()).collect();
            for name in names {
                if let Some((_, conn)) = self.connections.remove(&name) {
                    // 获取 stream 的所有权并 drop，这会关闭 socket
                    // Take ownership of stream and drop it, which closes the socket
                    drop(conn);
                    debug!(
                        "🔌 已关闭插件 {} 的连接 / Closed connection for plugin {}",
                        name, name
                    );
                }
            }

            info!("✅ 所有插件连接已关闭 / All plugin connections closed");
        } else {
            debug!("没有需要关闭的插件连接 / No plugin connections to close");
        }
    }

    /// 列出所有插件及其能力 / List all plugins and their capabilities
    pub fn list_plugins(&self) -> Vec<(String, Vec<String>)> {
        self.manager
            .plugins
            .iter()
            .map(|entry| {
                let name = entry.key().clone();
                let capabilities = entry.value().capabilities();
                (name, capabilities)
            })
            .collect()
    }

    /// 向插件发送 Protobuf 事件 / Send Protobuf event to plugin
    pub async fn send_event(
        &self,
        plugin_name: &str,
        event: &v::plugin::protocol::EventMessage,
    ) -> Result<v::plugin::protocol::EventResponse> {
        if let Some(conn) = self.connections.get(plugin_name) {
            let mut stream = conn.lock().await;

            // 发送 Protobuf 消息 / Send Protobuf message
            let bytes = event.encode_to_vec();
            stream.write_u32(bytes.len() as u32).await?;
            stream.write_all(&bytes).await?;
            stream.flush().await?;

            // 读取响应 / Read response
            let resp_len = stream.read_u32().await?;
            let mut resp_buf = vec![0u8; resp_len as usize];
            stream.read_exact(&mut resp_buf).await?;

            let response = v::plugin::protocol::EventResponse::decode(&resp_buf[..])?;
            Ok(response)
        } else {
            Err(anyhow::anyhow!("Plugin {} not found", plugin_name))
        }
    }

    /// 向插件发送事件（通用方法，返回 JSON）/ Send event to plugin (generic method, returns JSON)
    pub async fn send_event_with_payload(
        &self,
        plugin_name: &str,
        event_type: &str,
        payload: Vec<u8>,
    ) -> Result<Option<Value>> {
        let event = v::plugin::protocol::EventMessage {
            event_type: event_type.to_string(),
            payload,
            timestamp: chrono::Utc::now().timestamp_millis(),
            trace_id: String::new(),
        };
        
        match self.send_event(plugin_name, &event).await {
            Ok(response) => {
                // 将 Protobuf 响应的 data 解析为 JSON
                // Parse Protobuf response data as JSON
                if response.data.is_empty() {
                    Ok(Some(serde_json::json!({
                        "status": response.status,
                        "flow": response.flow
                    })))
                } else {
                    match serde_json::from_slice(&response.data) {
                        Ok(json) => Ok(Some(json)),
                        Err(_) => {
                            // 如果不是 JSON，返回状态
                            // If not JSON, return status
                            Ok(Some(serde_json::json!({
                                "status": response.status,
                                "flow": response.flow
                            })))
                        }
                    }
                }
            }
            Err(e) => {
                if e.to_string().contains("not found") {
                    Ok(None)
                } else {
                    Err(e)
                }
            }
        }
    }

    /// 广播消息事件到所有支持的插件 / Broadcast message event to all capable plugins
    pub async fn broadcast_message_event(&self, message: &Value) -> Result<Vec<(String, Value)>> {
        let mut responses = Vec::new();

        // 获取所有插件并按优先级排序 / Get all plugins and sort by priority
        let mut plugins: Vec<_> = self
            .manager
            .plugins
            .iter()
            .map(|entry| {
                let runtime = entry.value();
                (
                    entry.key().clone(),
                    runtime.priority(),
                    runtime.capabilities(),
                )
            })
            .collect();

        info!(
            "📋 发现 {} 个已注册插件 / Found {} registered plugins",
            plugins.len(),
            plugins.len()
        );

        // 按优先级降序排序 / Sort by priority descending
        plugins.sort_by(|a, b| b.1.cmp(&a.1));

        for (name, priority, capabilities) in plugins {
            debug!("🔍 检查插件 {} (优先级: {}, 能力: {:?}) / Checking plugin {} (priority: {}, capabilities: {:?})", 
                   name, priority, capabilities, name, priority, capabilities);

            // 检查插件是否支持 message 事件 / Check if plugin supports message events
            if !capabilities.iter().any(|cap| cap == "message") {
                debug!("⏭️  插件 {} 不支持 message 事件，跳过 / Plugin {} doesn't support message events, skipping", name, name);
                continue;
            }

            info!("📤 向插件 {} 发送 message.incoming 事件 / Sending message.incoming event to plugin {}", name, name);

            // 发送事件 / Send event
            // 将 JSON 转为字节 / Convert JSON to bytes
            let payload = serde_json::to_vec(message)?;
            match self.send_event_with_payload(&name, "message.incoming", payload).await {
                Ok(Some(response)) => {
                    info!(
                        "✅ 插件 {} 响应成功 / Plugin {} responded successfully",
                        name, name
                    );
                    debug!("Plugin {} response: {}", name, response);

                    // 检查是否需要停止传播 / Check if should stop propagation
                    if let Some(flow) = response.get("flow").and_then(|v| v.as_str()) {
                        if flow == "stop" {
                            info!("🛑 插件 {} 要求停止消息传播 / Plugin {} requested to stop message propagation", name, name);
                            responses.push((name, response));
                            break;
                        }
                    }

                    responses.push((name, response));
                }
                Ok(None) => {
                    warn!("⚠️  插件 {} 未连接 / Plugin {} not connected", name, name);
                }
                Err(e) => {
                    // 检查是否为连接断开错误 / Check if it's a connection broken error
                    let error_msg = e.to_string();
                    if error_msg.contains("Broken pipe") || error_msg.contains("Connection reset") {
                        warn!(
                            "⚠️  插件 {} 连接已断开（插件可能已退出）/ Plugin {} connection broken (plugin may have exited)",
                            name, name
                        );
                        // 从连接池移除该插件 / Remove plugin from connection pool
                        self.unregister(&name);
                        // 更新插件状态 / Update plugin status
                        if let Some(runtime) = self.manager.plugins.get(&name) {
                            runtime.set_status(PluginStatus::Stopped);
                        }
                    } else {
                        // 其他类型的错误记录为错误日志 / Log other types of errors as error
                        error!(
                            "❌ 向插件 {} 发送事件失败 / Error sending event to plugin {}: {}",
                            name, name, e
                        );
                    }
                }
            }
        }

        Ok(responses)
    }

    /// 发送存储事件到存储插件 / Send storage event to storage plugin
    ///
    /// 查找支持 storage 能力的插件并发送事件
    /// Find plugin that supports storage capability and send event
    ///
    /// # 参数 / Parameters
    /// - `event_type`: 存储事件类型 / Storage event type (e.g., "storage.message.save")
    /// - `payload`: 事件载荷数据 / Event payload data
    ///
    /// # 返回值 / Returns
    /// - `Ok(Some(response))`: 存储插件响应 / Storage plugin response
    /// - `Ok(None)`: 未找到存储插件 / Storage plugin not found
    /// - `Err(e)`: 发送失败 / Send failed
    pub async fn send_storage_event(
        &self,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>> {
        debug!("📦 发送存储事件 / Sending storage event: {}", event_type);

        // 查找存储插件 / Find storage plugin
        // 记录是否找到已安装但未就绪的存储插件 / Track if found installed but not ready storage plugin
        let mut found_installed_but_not_ready = false;

        for entry in self.manager.plugins.iter() {
            let runtime = entry.value();
            let plugin_name = entry.key();
            let status = runtime.status();
            let capabilities = runtime.capabilities();

            // 通过插件名称判断是否为存储插件 / Judge if it's a storage plugin by name
            let is_storage_plugin = plugin_name.contains("storage");

            // 如果是存储插件但状态不是 Running，说明已安装但未启动
            // If it's a storage plugin but status is not Running, it means installed but not started
            if is_storage_plugin && !matches!(status, PluginStatus::Running) {
                found_installed_but_not_ready = true;
                warn!(
                    "⚠️  存储插件 {} 已安装但未启动（状态: {:?}）/ Storage plugin {} is installed but not started (status: {:?})",
                    plugin_name, status, plugin_name, status
                );
                continue; // 继续查找其他可能的存储插件 / Continue to find other possible storage plugins
            }

            // 检查是否支持 storage 能力（插件已启动并完成握手）
            // Check if supports storage capability (plugin started and handshaked)
            if capabilities.iter().any(|cap| cap == "storage") {
                debug!("🎯 找到存储插件 / Found storage plugin: {}", plugin_name);

                // 发送事件到存储插件 / Send event to storage plugin
                // 将 JSON 转为字节 / Convert JSON to bytes
                let payload_bytes = serde_json::to_vec(payload)?;
                match self.send_event_with_payload(plugin_name, event_type, payload_bytes).await {
                    Ok(Some(response)) => {
                        debug!(
                            "✅ 存储插件响应成功 / Storage plugin responded: {:?}",
                            response
                        );
                        return Ok(Some(response));
                    }
                    Ok(None) => {
                        warn!(
                            "⚠️  存储插件 {} 未连接到连接池 / Storage plugin {} not connected to connection pool",
                            plugin_name, plugin_name
                        );
                        found_installed_but_not_ready = true;
                        continue; // 继续查找其他可能的存储插件 / Continue to find other possible storage plugins
                    }
                    Err(e) => {
                        // 检查是否为连接断开错误 / Check if it's a connection broken error
                        let error_msg = e.to_string();
                        if error_msg.contains("Broken pipe")
                            || error_msg.contains("Connection reset")
                        {
                            warn!(
                                "⚠️  存储插件 {} 连接已断开（插件可能已退出）/ Storage plugin {} connection broken (plugin may have exited)",
                                plugin_name, plugin_name
                            );
                            // 从连接池移除该插件 / Remove plugin from connection pool
                            self.unregister(plugin_name);
                            // 更新插件状态 / Update plugin status
                            if let Some(runtime) = self.manager.plugins.get(plugin_name) {
                                runtime.set_status(PluginStatus::Stopped);
                            }
                            found_installed_but_not_ready = true;
                            continue; // 继续查找其他可能的存储插件 / Continue to find other possible storage plugins
                        } else {
                            // 其他类型的错误直接返回 / Return other types of errors directly
                            error!(
                                "❌ 存储插件 {} 调用失败 / Storage plugin {} call failed: {}",
                                plugin_name, plugin_name, e
                            );
                            return Err(e);
                        }
                    }
                }
            }
        }

        // 根据情况给出不同的警告信息 / Give different warning messages based on the situation
        if found_installed_but_not_ready {
            warn!("⚠️  存储插件已安装但未就绪（未启动或未连接）/ Storage plugin installed but not ready (not started or not connected)");
        } else {
            warn!("⚠️  未找到存储插件（未安装）/ Storage plugin not found (not installed)");
        }

        Ok(None)
    }

    /// 保存消息到存储插件 / Save message to storage plugin
    pub async fn storage_save_message(
        &self,
        message_id: &str,
        from_uid: &str,
        to_uid: &str,
        content: &serde_json::Value,
        timestamp: i64,
        msg_type: &str,
        room_id: Option<&str>,
    ) -> Result<bool> {
        use prost::Message;
        use v::plugin::protocol::{SaveMessageRequest, SaveMessageResponse};

        // 构建 Protobuf 请求 / Build Protobuf request
        // 注意：room_id 暂时不在 Protobuf 定义中，可以放在 content 里
        let mut content_with_room = content.clone();
        if let Some(rid) = room_id {
            if let Some(obj) = content_with_room.as_object_mut() {
                obj.insert("room_id".to_string(), serde_json::Value::String(rid.to_string()));
            }
        }
        
        let request = SaveMessageRequest {
            message_id: message_id.to_string(),
            from_uid: from_uid.to_string(),
            to_uid: to_uid.to_string(),
            content: serde_json::to_string(&content_with_room)?,
            timestamp,
            msg_type: msg_type.to_string(),
        };

        // 查找存储插件 / Find storage plugin
        let storage_plugins: Vec<String> = self
            .list_plugins()
            .into_iter()
            .filter(|(_, caps)| caps.iter().any(|c| c == "storage"))
            .map(|(name, _)| name)
            .collect();

        if storage_plugins.is_empty() {
            return Ok(false);
        }

        let event = v::plugin::protocol::EventMessage {
            event_type: "storage.message.save".to_string(),
            payload: request.encode_to_vec(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            trace_id: message_id.to_string(),
        };

        match self
            .send_event(&storage_plugins[0], &event)
            .await
        {
            Ok(response) => {
                match SaveMessageResponse::decode(&response.data[..]) {
                    Ok(resp) => Ok(resp.status == "ok"),
                    Err(e) => {
                        warn!("存储插件响应解析失败 / Failed to parse storage plugin response: {}", e);
                        Ok(false)
                    }
                }
            }
            Err(e) => {
                warn!("存储插件调用失败 / Storage plugin call failed: {}", e);
                Ok(false)
            }
        }
    }

    /// 保存离线消息到存储插件 / Save offline message to storage plugin
    pub async fn storage_save_offline(
        &self,
        message_id: &str,
        from_uid: Option<&str>,
        to_uid: &str,
        content: &serde_json::Value,
        timestamp: i64,
        msg_type: &str,
        room_id: Option<&str>,
    ) -> Result<bool> {
        let payload = serde_json::json!({
            "message_id": message_id,
            "from_uid": from_uid,
            "to_uid": to_uid,
            "content": content,
            "timestamp": timestamp,
            "msg_type": msg_type,
            "room_id": room_id
        });

        match self
            .send_storage_event("storage.offline.save", &payload)
            .await
        {
            Ok(Some(response)) => {
                if response.get("status").and_then(|v| v.as_str()) == Some("ok") {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Ok(None) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// 拉取离线消息 / Pull offline messages
    pub async fn storage_pull_offline(
        &self,
        to_uid: &str,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>> {
        let payload = serde_json::json!({
            "to_uid": to_uid,
            "limit": limit
        });

        match self
            .send_storage_event("storage.offline.pull", &payload)
            .await
        {
            Ok(Some(response)) => {
                if let Some(messages) = response.get("messages").and_then(|v| v.as_array()) {
                    Ok(messages.clone())
                } else {
                    Ok(Vec::new())
                }
            }
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// 查询历史消息 / Query message history
    pub async fn storage_query_history(
        &self,
        uid: Option<&str>,
        peer: Option<&str>,
        since_ts: Option<i64>,
        until_ts: Option<i64>,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>> {
        let payload = serde_json::json!({
            "uid": uid,
            "peer": peer,
            "since_ts": since_ts,
            "until_ts": until_ts,
            "limit": limit
        });

        match self
            .send_storage_event("storage.message.history", &payload)
            .await
        {
            Ok(Some(response)) => {
                // 插件响应格式: {"status": "ok", "data": {"messages": [...], "count": N}}
                // Plugin response format: {"status": "ok", "data": {"messages": [...], "count": N}}
                let data = response.get("data").unwrap_or(&response);
                if let Some(messages) = data.get("messages").and_then(|v| v.as_array()) {
                    Ok(messages.clone())
                } else {
                    Ok(Vec::new())
                }
            }
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// 确认离线消息 / Acknowledge offline messages
    pub async fn storage_ack_offline(&self, to_uid: &str, message_ids: &[String]) -> Result<usize> {
        let payload = serde_json::json!({
            "to_uid": to_uid,
            "message_ids": message_ids
        });

        match self
            .send_storage_event("storage.offline.ack", &payload)
            .await
        {
            Ok(Some(response)) => {
                if let Some(removed) = response.get("removed").and_then(|v| v.as_u64()) {
                    Ok(removed as usize)
                } else {
                    Ok(0)
                }
            }
            Ok(None) => Ok(0),
            Err(e) => Err(e),
        }
    }

    /// 统计离线消息数量 / Count offline messages
    pub async fn storage_count_offline(&self, to_uid: &str) -> Result<usize> {
        let payload = serde_json::json!({
            "to_uid": to_uid
        });

        match self
            .send_storage_event("storage.offline.count", &payload)
            .await
        {
            Ok(Some(response)) => {
                if let Some(count) = response.get("count").and_then(|v| v.as_u64()) {
                    Ok(count as usize)
                } else {
                    Ok(0)
                }
            }
            Ok(None) => Ok(0),
            Err(e) => Err(e),
        }
    }

    /// 删除离线消息 / Delete offline messages
    pub async fn storage_delete_offline(
        &self,
        to_uid: &str,
        message_ids: &[String],
    ) -> Result<usize> {
        let payload = serde_json::json!({
            "to_uid": to_uid,
            "message_ids": message_ids
        });

        match self
            .send_storage_event("storage.offline.delete", &payload)
            .await
        {
            Ok(Some(response)) => {
                if let Some(deleted) = response.get("deleted").and_then(|v| v.as_u64()) {
                    Ok(deleted as usize)
                } else {
                    Ok(0)
                }
            }
            Ok(None) => Ok(0),
            Err(e) => Err(e),
        }
    }

    // ==================== 插件间通信功能 / Inter-Plugin Communication ====================

    /// 插件 A 直接调用插件 B / Plugin A directly calls Plugin B
    ///
    /// # 参数 / Parameters
    /// - `from_plugin`: 发送方插件名称 / Sender plugin name
    /// - `to_plugin`: 接收方插件名称 / Receiver plugin name
    /// - `method`: 调用的方法名 / Method name to call
    /// - `params`: 方法参数 / Method parameters
    ///
    /// # 返回值 / Returns
    /// - `Ok(Some(response))`: 目标插件响应 / Target plugin response
    /// - `Ok(None)`: 目标插件未连接 / Target plugin not connected
    /// - `Err(e)`: 调用失败 / Call failed
    ///
    /// # 示例 / Example
    /// ```rust
    /// let response = pool.plugin_call(
    ///     "plugin_a",
    ///     "plugin_b",
    ///     "process_data",
    ///     &json!({"data": "hello"})
    /// ).await?;
    /// ```
    pub async fn plugin_call(
        &self,
        from_plugin: &str,
        to_plugin: &str,
        method: &str,
        params: &Value,
    ) -> Result<Option<Value>> {
        // 验证发送方插件存在 / Verify sender exists
        if !self.connections.contains_key(from_plugin) {
            return Err(anyhow!("Sender plugin not connected: {}", from_plugin));
        }

        info!(
            "🔗 插件调用 / Plugin call: {} -> {} (method: {})",
            from_plugin, to_plugin, method
        );

        // 构建插件间调用事件 / Build inter-plugin call event
        let event_type = format!("plugin.call.{}", method);
        let enriched_payload = serde_json::json!({
            "from_plugin": from_plugin,
            "method": method,
            "params": params
        });

        // 向目标插件发送事件 / Send event to target plugin
        match self
            .send_event_with_payload(to_plugin, &event_type, serde_json::to_vec(&enriched_payload)?)
            .await
        {
            Ok(Some(response)) => {
                info!(
                    "✅ 插件调用成功 / Plugin call succeeded: {} -> {}",
                    from_plugin, to_plugin
                );
                Ok(Some(response))
            }
            Ok(None) => {
                warn!(
                    "⚠️  目标插件未连接 / Target plugin not connected: {}",
                    to_plugin
                );
                Ok(None)
            }
            Err(e) => {
                error!(
                    "❌ 插件调用失败 / Plugin call failed: {} -> {}: {}",
                    from_plugin, to_plugin, e
                );
                Err(e)
            }
        }
    }

    /// 插件间点对点消息传递 / Point-to-point message between plugins
    ///
    /// # 参数 / Parameters
    /// - `from_plugin`: 发送方插件名称 / Sender plugin name
    /// - `to_plugin`: 接收方插件名称 / Receiver plugin name
    /// - `message`: 消息内容 / Message content
    ///
    /// # 返回值 / Returns
    /// - `Ok(true)`: 消息已送达 / Message delivered
    /// - `Ok(false)`: 目标插件未连接 / Target plugin not connected
    /// - `Err(e)`: 发送失败 / Send failed
    ///
    /// # 示例 / Example
    /// ```rust
    /// pool.plugin_send_message(
    ///     "plugin_a",
    ///     "plugin_b",
    ///     &json!({"type": "notification", "content": "hello"})
    /// ).await?;
    /// ```
    pub async fn plugin_send_message(
        &self,
        from_plugin: &str,
        to_plugin: &str,
        message: &Value,
    ) -> Result<bool> {
        // 验证发送方插件存在 / Verify sender exists
        if !self.connections.contains_key(from_plugin) {
            return Err(anyhow!("Sender plugin not connected: {}", from_plugin));
        }

        info!(
            "📨 插件消息 / Plugin message: {} -> {}",
            from_plugin, to_plugin
        );

        // 构建插件间消息事件 / Build inter-plugin message event
        let enriched_message = serde_json::json!({
            "from_plugin": from_plugin,
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "message": message
        });

        // 发送到目标插件 / Send to target plugin
        match self
            .send_event_with_payload(to_plugin, "plugin.message", serde_json::to_vec(&enriched_message)?)
            .await
        {
            Ok(Some(_)) => {
                info!(
                    "✅ 插件消息已送达 / Plugin message delivered: {} -> {}",
                    from_plugin, to_plugin
                );
                Ok(true)
            }
            Ok(None) => {
                warn!(
                    "⚠️  目标插件未连接 / Target plugin not connected: {}",
                    to_plugin
                );
                Ok(false)
            }
            Err(e) => {
                error!(
                    "❌ 插件消息发送失败 / Plugin message send failed: {} -> {}: {}",
                    from_plugin, to_plugin, e
                );
                Err(e)
            }
        }
    }

    /// 插件广播消息到其他插件 / Plugin broadcasts message to other plugins
    ///
    /// # 参数 / Parameters
    /// - `from_plugin`: 发送方插件名称 / Sender plugin name
    /// - `message`: 广播消息内容 / Broadcast message content
    /// - `filter_capabilities`: 可选的能力过滤器 / Optional capability filter
    ///
    /// # 返回值 / Returns
    /// - `Ok(responses)`: 所有接收插件的响应列表 / List of responses from all receivers
    ///
    /// # 示例 / Example
    /// ```rust
    /// // 广播给所有插件 / Broadcast to all plugins
    /// let responses = pool.plugin_broadcast(
    ///     "plugin_a",
    ///     &json!({"event": "data_updated"}),
    ///     None
    /// ).await?;
    ///
    /// // 只广播给支持特定能力的插件 / Broadcast only to plugins with specific capabilities
    /// let responses = pool.plugin_broadcast(
    ///     "plugin_a",
    ///     &json!({"event": "data_updated"}),
    ///     Some(vec!["storage".to_string()])
    /// ).await?;
    /// ```
    pub async fn plugin_broadcast(
        &self,
        from_plugin: &str,
        message: &Value,
        filter_capabilities: Option<Vec<String>>,
    ) -> Result<Vec<(String, Value)>> {
        // 验证发送方插件存在 / Verify sender exists
        if !self.connections.contains_key(from_plugin) {
            return Err(anyhow!("Sender plugin not connected: {}", from_plugin));
        }

        info!(
            "📢 插件广播 / Plugin broadcast from: {} (filter: {:?})",
            from_plugin, filter_capabilities
        );

        let mut responses = Vec::new();

        // 构建广播消息 / Build broadcast message
        let enriched_message = serde_json::json!({
            "from_plugin": from_plugin,
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "message": message
        });

        // 遍历所有已连接的插件 / Iterate all connected plugins
        for entry in self.connections.iter() {
            let plugin_name = entry.key();

            // 跳过发送方自己 / Skip sender itself
            if plugin_name == from_plugin {
                continue;
            }

            // 能力过滤 / Filter by capabilities
            if let Some(caps) = &filter_capabilities {
                if let Some(runtime) = self.manager.plugins.get(plugin_name.as_str()) {
                    let plugin_caps = runtime.capabilities();
                    if !caps.iter().any(|c| plugin_caps.contains(c)) {
                        debug!(
                            "⏭️  跳过插件 {} (不满足能力要求) / Skip plugin {} (capability mismatch)",
                            plugin_name, plugin_name
                        );
                        continue;
                    }
                }
            }

            // 发送广播事件 / Send broadcast event
            match self
                .send_event_with_payload(plugin_name, "plugin.broadcast", serde_json::to_vec(&enriched_message)?)
                .await
            {
                Ok(Some(response)) => {
                    info!(
                        "✅ 插件 {} 收到广播 / Plugin {} received broadcast",
                        plugin_name, plugin_name
                    );
                    responses.push((plugin_name.clone(), response));
                }
                Ok(None) => {
                    debug!(
                        "⚠️  插件 {} 未连接 / Plugin {} not connected",
                        plugin_name, plugin_name
                    );
                }
                Err(e) => {
                    warn!(
                        "⚠️  向插件 {} 广播失败 / Broadcast to plugin {} failed: {}",
                        plugin_name, plugin_name, e
                    );
                }
            }
        }

        info!(
            "📊 广播完成 / Broadcast completed: {} 个插件响应 / {} plugins responded",
            responses.len(),
            responses.len()
        );

        Ok(responses)
    }

    /// 添加房间成员 / Add room member
    pub async fn storage_add_room_member(&self, room_id: &str, uid: &str) -> Result<bool> {
        let payload = serde_json::json!({
            "room_id": room_id,
            "uid": uid
        });

        match self
            .send_storage_event("storage.room.add_member", &payload)
            .await
        {
            Ok(Some(response)) => Ok(response.get("status").and_then(|v| v.as_str()) == Some("ok")),
            Ok(None) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// 移除房间成员 / Remove room member
    pub async fn storage_remove_room_member(&self, room_id: &str, uid: &str) -> Result<bool> {
        let payload = serde_json::json!({
            "room_id": room_id,
            "uid": uid
        });

        match self
            .send_storage_event("storage.room.remove_member", &payload)
            .await
        {
            Ok(Some(response)) => Ok(response.get("status").and_then(|v| v.as_str()) == Some("ok")),
            Ok(None) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// 列出房间成员 / List room members
    pub async fn storage_list_room_members(&self, room_id: &str) -> Result<Vec<String>> {
        let payload = serde_json::json!({
            "room_id": room_id
        });

        match self
            .send_storage_event("storage.room.list_members", &payload)
            .await
        {
            Ok(Some(response)) => {
                if let Some(members) = response.get("members").and_then(|v| v.as_array()) {
                    Ok(members
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect())
                } else {
                    Ok(Vec::new())
                }
            }
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// 列出所有房间 / List all rooms
    pub async fn storage_list_rooms(&self) -> Result<Vec<String>> {
        let payload = serde_json::json!({});

        match self.send_storage_event("storage.room.list", &payload).await {
            Ok(Some(response)) => {
                if let Some(rooms) = response.get("rooms").and_then(|v| v.as_array()) {
                    Ok(rooms
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect())
                } else {
                    Ok(Vec::new())
                }
            }
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// 记录已读回执 / Record read receipt
    pub async fn storage_record_read(
        &self,
        uid: &str,
        message_id: &str,
        timestamp: i64,
    ) -> Result<bool> {
        let payload = serde_json::json!({
            "uid": uid,
            "message_id": message_id,
            "timestamp": timestamp
        });

        match self
            .send_storage_event("storage.read.record", &payload)
            .await
        {
            Ok(Some(response)) => Ok(response.get("status").and_then(|v| v.as_str()) == Some("ok")),
            Ok(None) => Ok(false),
            Err(e) => Err(e),
        }
    }
}
