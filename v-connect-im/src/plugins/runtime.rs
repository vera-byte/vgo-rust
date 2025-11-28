//! 插件运行时管理模块 / Plugin runtime management module
//!
//! 负责插件的加载、启动、停止和通信
//! Responsible for plugin loading, starting, stopping and communication

use anyhow::{anyhow, Result};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::{Child, Command};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use v::plugin::installer::PluginInstaller;

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
        }
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
}

/// 插件元数据 / Plugin metadata
#[derive(Clone, Default)]
struct PluginMetadata {
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
        }
    }

    /// 设置全局 socket 路径（所有插件共享）/ Set global socket path shared by all plugins
    pub fn set_global_socket_path(&mut self, path: impl AsRef<Path>) {
        self.global_socket_path = Some(path.as_ref().to_path_buf());
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
        // 检查是否已存在 / Check if already exists
        if let Some(runtime) = self.plugins.get(name) {
            let status = runtime.status();
            if matches!(status, PluginStatus::Running | PluginStatus::Starting) {
                warn!("Plugin {} is already running", name);
                return Ok(());
            }
        }

        // 查找插件二进制文件 / Find plugin binary
        let plugin_path = self.find_plugin_binary(name)?;
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
        let mut cmd = Command::new(&runtime.path);
        cmd.arg("--socket")
            .arg(socket_path.to_string_lossy().as_ref())
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

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
            runtime.set_status(PluginStatus::Stopping);

            // 终止进程 / Terminate process
            if let Some(mut child) = {
                let mut guard = runtime.process.write();
                guard.take()
            } {
                if let Err(e) = child.kill().await {
                    error!("Failed to kill plugin {}: {}", name, e);
                } else if let Err(e) = child.wait().await {
                    error!("Failed to wait plugin {} exit: {}", name, e);
                }
            }

            // 清理 socket / Cleanup socket
            if let Some(socket_path) = &runtime.socket_path {
                let _ = std::fs::remove_file(socket_path);
            }

            runtime.set_status(PluginStatus::Stopped);
            self.plugins.remove(name);
            info!("Plugin {} stopped", name);
            Ok(())
        } else {
            Err(anyhow!("Plugin {} not found", name))
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

        for name in names {
            if let Err(e) = self.stop_plugin(&name).await {
                error!("Failed to stop plugin {}: {}", name, e);
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
                if let Some(version) = value
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                {
                    return PluginMetadata {
                        version: Some(version),
                    };
                }
            }
        }
        PluginMetadata::default()
    }
}

/// Unix Socket 服务器 / Unix Socket server
pub struct UnixSocketServer {
    listener: UnixListener,
    plugin_manager: Arc<PluginRuntimeManager>,
}

impl UnixSocketServer {
    /// 创建并启动 Unix Socket 服务器 / Create and start Unix Socket server
    pub async fn new(
        socket_path: impl AsRef<Path>,
        plugin_manager: Arc<PluginRuntimeManager>,
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

        Ok(Self {
            listener,
            plugin_manager,
        })
    }

    /// 运行服务器 / Run server
    pub async fn run(&self) -> Result<()> {
        loop {
            match self.listener.accept().await {
                Ok((stream, _)) => {
                    let manager = self.plugin_manager.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, manager).await {
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
    }

    /// 处理连接 / Handle connection
    async fn handle_connection(
        mut stream: UnixStream,
        manager: Arc<PluginRuntimeManager>,
    ) -> Result<()> {
        let mut handshake_done = false;
        loop {
            match stream.read_u32().await {
                Ok(len) => {
                    let mut buffer = vec![0u8; len as usize];
                    if let Err(e) = stream.read_exact(&mut buffer).await {
                        error!("Plugin connection read error: {}", e);
                        break;
                    }

                    let payload: Value = serde_json::from_slice(&buffer).unwrap_or(Value::Null);
                    if !handshake_done {
                        handshake_done = true;
                        if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
                            info!("🤝 Plugin handshake received: {}", name);
                        } else {
                            info!("🤝 Plugin handshake received (unknown name)");
                        }
                    } else {
                        debug!("📨 Plugin message: {}", payload);
                    }

                    let response = serde_json::to_vec(&serde_json::json!({
                        "status": "ok"
                    }))?;
                    stream.write_u32(response.len() as u32).await?;
                    stream.write_all(&response).await?;
                    stream.flush().await?;
                }
                Err(e) => {
                    // 连接关闭（EOF常见于优雅停机）/ Connection closed (EOF common on graceful shutdown)
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        info!("Plugin connection closed gracefully (EOF): {}", e);
                    } else {
                        debug!("Plugin connection closed: {}", e);
                    }
                    break;
                }
            }
        }

        drop(manager);
        Ok(())
    }
}
