//! 插件协议处理器 / Plugin protocol handler
//!
//! 支持 Protobuf 协议的服务端实现 / Server-side Protobuf protocol support

use anyhow::Result;
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tracing::{debug, info};

use v::plugin::protocol::{
    negotiate_protocol, EventMessage, EventResponse, HandshakeRequest, HandshakeResponse,
    ProtocolFormat,
};

/// 协议处理会话 / Protocol handler session
pub struct ProtocolSession {
    stream: UnixStream,
    protocol: ProtocolFormat,
    plugin_name: Option<String>,
}

impl ProtocolSession {
    /// 创建新会话（默认 Protobuf）/ Create new session (default Protobuf)
    pub fn new(stream: UnixStream) -> Self {
        let protocol = ProtocolFormat::Protobuf;
        Self {
            stream,
            protocol,
            plugin_name: None,
        }
    }

    /// 处理握手 / Handle handshake
    pub async fn handle_handshake(&mut self) -> Result<HandshakeRequest> {
        // 读取握手消息 / Read handshake message
        let len = self.stream.read_u32().await?;
        let mut buffer = vec![0u8; len as usize];
        self.stream.read_exact(&mut buffer).await?;

        // 使用 prost 解码握手请求 / Decode handshake request using prost
        let request = HandshakeRequest::decode(buffer.as_slice())?;
        self.plugin_name = Some(request.name.clone());

        info!(
            "🤝 Plugin handshake: {} v{} (priority: {}, protocol: {}, capabilities: {:?})",
            request.name, request.version, request.priority, request.protocol, request.capabilities
        );

        // 协议协商 / Protocol negotiation
        let negotiated = negotiate_protocol(&request.protocol);
        if negotiated != self.protocol {
            info!(
                "🔄 Protocol negotiated: {:?} -> {:?}",
                self.protocol, negotiated
            );
            self.protocol = negotiated;
        }

        // 发送握手响应 / Send handshake response
        let response = HandshakeResponse {
            status: "ok".to_string(),
            message: String::new(),
            config: String::new(),
            protocol: format!("{:?}", self.protocol).to_lowercase(),
        };

        // 使用 prost 编码握手响应 / Encode handshake response using prost
        let resp_bytes = response.encode_to_vec();
        self.stream.write_u32(resp_bytes.len() as u32).await?;
        self.stream.write_all(&resp_bytes).await?;
        self.stream.flush().await?;

        Ok(request)
    }

    /// 发送事件 / Send event
    pub async fn send_event(
        &mut self,
        event_type: &str,
        payload: Vec<u8>,
    ) -> Result<EventResponse> {
        // 构建事件消息 / Build event message
        let event = EventMessage {
            event_type: event_type.to_string(),
            payload,
            timestamp: chrono::Utc::now().timestamp_millis(),
            trace_id: String::new(),
        };

        // 使用 prost 编码事件 / Encode event using prost
        let bytes = event.encode_to_vec();

        // 发送消息 / Send message
        self.stream.write_u32(bytes.len() as u32).await?;
        self.stream.write_all(&bytes).await?;
        self.stream.flush().await?;

        debug!(
            "[plugin:{}] sent event: {} (protocol: {:?}, size: {} bytes)",
            self.plugin_name.as_deref().unwrap_or("unknown"),
            event_type,
            self.protocol,
            bytes.len()
        );

        // 读取响应 / Read response
        let resp_len = self.stream.read_u32().await?;
        let mut resp_buf = vec![0u8; resp_len as usize];
        self.stream.read_exact(&mut resp_buf).await?;

        // 使用 prost 解码响应 / Decode response using prost
        let response = EventResponse::decode(resp_buf.as_slice())?;

        debug!(
            "[plugin:{}] received response: status={}, flow={} (size: {} bytes)",
            self.plugin_name.as_deref().unwrap_or("unknown"),
            response.status,
            response.flow,
            resp_buf.len()
        );

        Ok(response)
    }

    /// 获取插件名称 / Get plugin name
    pub fn plugin_name(&self) -> Option<&str> {
        self.plugin_name.as_deref()
    }

    /// 获取当前协议 / Get current protocol
    pub fn protocol(&self) -> ProtocolFormat {
        self.protocol
    }

    /// 分离 stream / Split stream
    pub fn into_stream(self) -> UnixStream {
        self.stream
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_protocol_negotiation() {
        let protocol = negotiate_protocol("protobuf");
        assert_eq!(protocol, ProtocolFormat::Protobuf);
    }
}
