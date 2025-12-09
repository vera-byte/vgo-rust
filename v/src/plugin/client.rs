//! 插件客户端 - Protobuf 协议 / Plugin client - Protobuf protocol

use anyhow::Result;
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::watch;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

use super::protocol::{
    negotiate_protocol, EventMessage, EventResponse, HandshakeRequest, HandshakeResponse,
    ProtocolFormat,
};

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
    /// 协议格式（仅支持 Protobuf）/ Protocol format (Protobuf only)
    fn protocol(&self) -> ProtocolFormat {
        ProtocolFormat::Protobuf
    }
    /// 应用配置 / Apply configuration
    fn config(&mut self, _cfg: &str) -> Result<()> {
        Ok(())
    }
    /// 处理事件并返回响应 / Handle event and return response
    fn on_event(&mut self, event: &EventMessage) -> Result<EventResponse>;
}

/// 插件客户端（Protobuf 协议）/ Plugin client (Protobuf protocol)
pub struct PluginClient<H: PluginHandler> {
    socket_path: String,
    handler: H,
    reconnect_backoff: (u64, u64),      // (initial_ms, max_ms)
    ident: String,                      // 插件标识（名称-版本）/ Plugin identifier (name-version)
    shutdown_tx: watch::Sender<bool>,   // 关闭信号发送器 / Shutdown signal sender
    shutdown_rx: watch::Receiver<bool>, // 关闭信号接收器 / Shutdown signal receiver
    protocol: ProtocolFormat,           // 当前使用的协议 / Current protocol
}

impl<H: PluginHandler> PluginClient<H> {
    /// 创建客户端 / Create client
    pub fn new(socket_path: impl Into<String>, handler: H) -> Self {
        let socket = socket_path.into();
        let ident = format!("{}-{}", handler.name(), handler.version());
        let protocol = handler.protocol();

        info!(
            "[plugin:{}] init client, socket={}, protocol={:?}",
            ident, socket, protocol
        );

        let (tx, rx) = watch::channel(false);
        Self {
            socket_path: socket,
            handler,
            reconnect_backoff: (500, 5000),
            ident,
            shutdown_tx: tx,
            shutdown_rx: rx,
            protocol,
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
        let handshake = HandshakeRequest {
            name: self.handler.name().to_string(),
            version: self.handler.version().to_string(),
            capabilities: self.handler.capabilities(),
            priority: self.handler.priority(),
            protocol: format!("{:?}", self.protocol).to_lowercase(),
        };

        // 使用 prost 编码握手消息 / Encode handshake using prost
        let bytes = handshake.encode_to_vec();

        // 发送消息 / Send message
        stream.write_u32(bytes.len() as u32).await?;
        stream.write_all(&bytes).await?;
        stream.flush().await?;

        // 打印插件信息 / Print plugin info
        info!("");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!("🔌 Plugin Information / 插件信息");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!("  Plugin ID      : {}", handshake.name);
        info!("  Version        : {}", handshake.version);
        info!("  Priority       : {}", handshake.priority);
        info!("  Protocol       : {:?}", self.protocol);
        info!("  Capabilities   : [{}]", handshake.capabilities.join(", "));
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!("");

        // 读取响应 / Read response
        let resp_len = stream.read_u32().await?;
        let mut resp = vec![0u8; resp_len as usize];
        stream.read_exact(&mut resp).await?;

        // 使用 prost 解码握手响应 / Decode handshake response using prost
        let resp_val = HandshakeResponse::decode(resp.as_slice())?;

        if resp_val.status == "ok" {
            info!("✅ Handshake successful / 握手成功");

            // 协议协商 / Protocol negotiation
            if !resp_val.protocol.is_empty() {
                let negotiated = negotiate_protocol(&resp_val.protocol);
                if negotiated != self.protocol {
                    info!(
                        "🔄 Protocol negotiated: {:?} -> {:?}",
                        self.protocol, negotiated
                    );
                    self.protocol = negotiated;
                }
            }
        } else {
            warn!("⚠️  Handshake response: {:?}", resp_val);
        }

        if !resp_val.config.is_empty() {
            let _ = self.handler.config(&resp_val.config);
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
                    // 读取消息 / Read message
                    let len = stream.read_u32().await?;
                    let mut buffer = vec![0u8; len as usize];
                    stream.read_exact(&mut buffer).await?;

                    // 使用 prost 解码事件 / Decode event using prost
                    let event = EventMessage::decode(buffer.as_slice())?;

                    debug!(
                        "[plugin:{}] event: {} (payload size: {} bytes)",
                        self.ident, event.event_type, event.payload.len()
                    );

                    // 处理事件 / Handle event
                    let response = self.handler.on_event(&event)?;

                    // 使用 prost 编码响应 / Encode response using prost
                    let resp_bytes = response.encode_to_vec();

                    // 发送响应 / Send response
                    stream.write_u32(resp_bytes.len() as u32).await?;
                    stream.write_all(&resp_bytes).await?;
                    stream.flush().await?;

                    debug!("[plugin:{}] response sent", self.ident);
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
