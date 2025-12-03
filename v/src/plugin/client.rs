use anyhow::Result;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::watch;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

/// 插件事件处理接口 / Plugin event handler interface
pub trait PluginHandler {
    /// 插件名称 / Plugin name
    fn name(&self) -> &'static str;
    /// 插件版本 / Plugin version
    fn version(&self) -> &'static str;
    /// 能力声明（必须实现）/ Capability declaration (required)
    fn capabilities(&self) -> Vec<String>;
    /// 插件优先级 / Plugin priority
    fn priority(&self) -> i32 {
        0
    }
    /// 应用配置 / Apply configuration
    fn config(&mut self, _cfg: &Value) -> Result<()> {
        Ok(())
    }
    /// 处理事件并返回响应 / Handle event and return response
    fn on_event(&mut self, event_type: &str, payload: &Value) -> Result<Value>;
}

/// 插件客户端（Unix Socket）/ Plugin client (Unix Socket)
pub struct PluginClient<H: PluginHandler> {
    socket_path: String,
    handler: H,
    reconnect_backoff: (u64, u64),      // (initial_ms, max_ms)
    ident: String,                      // 插件标识（名称-版本）/ Plugin identifier (name-version)
    shutdown_tx: watch::Sender<bool>,   // 关闭信号发送器 / Shutdown signal sender
    shutdown_rx: watch::Receiver<bool>, // 关闭信号接收器 / Shutdown signal receiver
}

impl<H: PluginHandler> PluginClient<H> {
    /// 创建客户端 / Create client
    pub fn new(socket_path: impl Into<String>, handler: H) -> Self {
        let socket = socket_path.into();
        let ident = format!("{}-{}", handler.name(), handler.version());
        // 初始化客户端并记录插件标识 / Initialize client and record plugin identifier
        info!("[plugin:{}] init client, socket={}", ident, socket);
        let (tx, rx) = watch::channel(false);
        Self {
            socket_path: socket,
            handler,
            reconnect_backoff: (500, 5000),
            ident,
            shutdown_tx: tx,
            shutdown_rx: rx,
        }
    }

    /// 触发关闭信号 / Trigger shutdown signal
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    /// 运行并监听 Ctrl-C 以退出 / Run and listen Ctrl-C to exit
    pub async fn run_forever_with_ctrlc(&mut self) -> Result<()> {
        tokio::select! {
            res = self.run_forever() => res,
            _ = tokio::signal::ctrl_c() => {
                info!("[plugin:{}] ctrl-c received, shutting down", self.ident);
                self.shutdown();
                Ok(())
            }
        }
    }

    /// 永久运行，自动重连 / Run forever with auto reconnect
    pub async fn run_forever(&mut self) -> Result<()> {
        let mut backoff = self.reconnect_backoff.0;
        loop {
            // 如果收到关闭信号则退出 / Exit on shutdown signal
            if *self.shutdown_rx.borrow() {
                info!("[plugin:{}] shutdown flag set, exiting", self.ident);
                break;
            }
            match self.run_once().await {
                Ok(_) => {
                    info!("[plugin:{}] session finished, reconnecting", self.ident);
                    backoff = self.reconnect_backoff.0;
                }
                Err(e) => {
                    warn!("[plugin:{}] session error: {}", self.ident, e);
                    tokio::select! {
                        _ = sleep(Duration::from_millis(backoff)) => {},
                        _ = self.shutdown_rx.changed() => {
                            if *self.shutdown_rx.borrow() { break; }
                        }
                    }
                    backoff = std::cmp::min(backoff * 2, self.reconnect_backoff.1);
                }
            }
        }
        Ok(())
    }

    /// 单次会话 / Single session
    async fn run_once(&mut self) -> Result<()> {
        self.wait_for_socket().await?;
        info!(
            "[plugin:{}] connecting socket: {}",
            self.ident, self.socket_path
        );
        let mut stream = self.connect_with_retry().await?;
        info!("[plugin:{}] connected", self.ident);
        self.send_handshake(&mut stream).await?;
        self.listen_loop(&mut stream).await
    }

    /// 等待 socket 文件 / Wait for socket file
    async fn wait_for_socket(&mut self) -> Result<()> {
        let mut retries = 120u32;
        while !std::path::Path::new(&self.socket_path).exists() {
            if retries == 0 {
                error!(
                    "[plugin:{}] socket not found: {}",
                    self.ident, self.socket_path
                );
                return Err(anyhow::anyhow!("socket not found"));
            }
            debug!(
                "[plugin:{}] waiting for socket: {} (retries: {})",
                self.ident, self.socket_path, retries
            );
            retries -= 1;
            tokio::select! {
                _ = sleep(Duration::from_millis(500)) => {},
                _ = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        warn!("[plugin:{}] shutdown during wait_for_socket", self.ident);
                        return Err(anyhow::anyhow!("shutdown"));
                    }
                }
            }
        }
        Ok(())
    }

    /// 带重试的连接（处理连接拒绝）/ Connect with retry (handle ECONNREFUSED)
    async fn connect_with_retry(&mut self) -> Result<UnixStream> {
        use std::io::ErrorKind;
        let mut rx = self.shutdown_rx.clone();
        let mut backoff = self.reconnect_backoff.0.min(500);
        loop {
            tokio::select! {
                res = UnixStream::connect(&self.socket_path) => {
                    match res {
                        Ok(stream) => return Ok(stream),
                        Err(e) => {
                            if e.kind() == ErrorKind::ConnectionRefused {
                                warn!("[plugin:{}] connect refused, retrying", self.ident);
                                tokio::select! {
                                    _ = sleep(Duration::from_millis(backoff)) => {},
                                    _ = rx.changed() => {
                                        if *rx.borrow() { return Err(anyhow::anyhow!("shutdown")); }
                                    }
                                }
                                backoff = std::cmp::min(backoff * 2, self.reconnect_backoff.1);
                                continue;
                            } else {
                                return Err(e.into());
                            }
                        }
                    }
                }
                _ = rx.changed() => {
                    if *rx.borrow() { return Err(anyhow::anyhow!("shutdown")); }
                }
            }
        }
    }

    /// 发送握手信息 / Send handshake info
    async fn send_handshake(&mut self, stream: &mut UnixStream) -> Result<()> {
        let info = serde_json::json!({
            "name": self.handler.name(),
            "version": self.handler.version(),
            "capabilities": self.handler.capabilities(),
            "priority": self.handler.priority(),
        });
        let bytes = serde_json::to_vec(&info)?;
        stream.write_u32(bytes.len() as u32).await?;
        stream.write_all(&bytes).await?;
        stream.flush().await?;
        info!("[plugin:{}] handshake sent: {}", self.ident, info);
        let resp_len = stream.read_u32().await?;
        let mut resp = vec![0u8; resp_len as usize];
        stream.read_exact(&mut resp).await?;
        let resp_val: Value = serde_json::from_slice(&resp)?;
        info!("[plugin:{}] handshake ack: {}", self.ident, resp_val);
        if let Some(cfg) = resp_val.get("config") {
            let _ = self.handler.config(cfg);
            debug!("[plugin:{}] config applied from handshake", self.ident);
        }
        Ok(())
    }

    /// 事件循环 / Event loop
    async fn listen_loop(&mut self, stream: &mut UnixStream) -> Result<()> {
        loop {
            tokio::select! {
                _ = self.shutdown_rx.changed() => {
                    if *self.shutdown_rx.borrow() {
                        info!("[plugin:{}] shutdown received in listen_loop", self.ident);
                        break;
                    }
                }
                result = async {
                    let len = stream.read_u32().await?;
                    let mut buffer = vec![0u8; len as usize];
                    stream.read_exact(&mut buffer).await?;
                    let msg: Value = serde_json::from_slice(&buffer)?;
                    let event_type = msg
                        .get("event_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let payload = msg.get("payload").cloned().unwrap_or(Value::Null);
                    debug!("[plugin:{}] event: {} payload={}", self.ident, event_type, payload);
                    let resp = self.handler.on_event(event_type, &payload)?;
                    let resp_bytes = serde_json::to_vec(&resp)?;
                    stream.write_u32(resp_bytes.len() as u32).await?;
                    stream.write_all(&resp_bytes).await?;
                    stream.flush().await?;
                    debug!("[plugin:{}] response sent: {}", self.ident, resp);
                    Ok::<(), anyhow::Error>(())
                } => {
                    if let Err(e) = result {
                        return Err(e);
                    }
                }
            }
        }
        Ok(())
    }
}
