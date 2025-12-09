//! 插件通信协议 - Protocol Buffers
//! Plugin communication protocol - Protocol Buffers only

/// 协议格式（仅支持 Protobuf）/ Protocol format (Protobuf only)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolFormat {
    /// Protocol Buffers（高性能二进制协议）/ Protocol Buffers (high-performance binary protocol)
    Protobuf,
}

impl Default for ProtocolFormat {
    fn default() -> Self {
        Self::Protobuf
    }
}

// 重新导出 proto 生成的类型 / Re-export proto generated types
pub use super::proto::{
    AckOfflineMessagesRequest,
    AckOfflineMessagesResponse,
    AddRoomMemberRequest,
    AddRoomMemberResponse,
    BanUserRequest,
    BanUserResponse,
    CountOfflineMessagesRequest,
    CountOfflineMessagesResponse,
    EventMessage,
    EventResponse,

    GetRoomMembersRequest,
    GetRoomMembersResponse,

    // 基础消息 / Basic messages
    HandshakeRequest,
    HandshakeResponse,
    HealthCheckRequest,
    HealthCheckResponse,
    // 网关插件消息 / Gateway plugin messages
    HttpRequest,
    HttpResponse,
    KickOutRequest,
    KickOutResponse,
    // 认证插件消息 / Authentication plugin messages
    LoginRequest,
    LoginResponse,
    LogoutRequest,
    LogoutResponse,
    OfflineMessage,
    ProxyRequest,
    ProxyResponse,
    PullOfflineMessagesRequest,
    PullOfflineMessagesResponse,
    RegisterRouteRequest,
    RegisterRouteResponse,
    RemoveRoomMemberRequest,
    RemoveRoomMemberResponse,
    RenewTokenRequest,
    RenewTokenResponse,
    // 存储插件消息 / Storage plugin messages
    SaveMessageRequest,
    SaveMessageResponse,
    SaveOfflineMessageRequest,
    SaveOfflineMessageResponse,
    TokenReplacedRequest,
    TokenReplacedResponse,
    UnregisterRouteRequest,
    UnregisterRouteResponse,
    ValidateTokenRequest,
    ValidateTokenResponse,

    WebSocketMessage,
    WebSocketResponse,
};

/// 协议协商（仅支持 Protobuf）/ Protocol negotiation (Protobuf only)
pub fn negotiate_protocol(_client_protocol: &str) -> ProtocolFormat {
    ProtocolFormat::Protobuf
}
