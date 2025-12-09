# Protobuf 协议定义

## 目录结构

```
proto/
├── README.md                    # 本文件
├── base.proto                   # 基础协议（握手、事件）
└── storage/                     # 存储插件协议
    └── storage.proto            # 存储相关消息定义
```

## 文件说明

### base.proto - 基础协议

**用途：** 插件与主服务之间的基础通信协议

**包含：**
- `HandshakeRequest` / `HandshakeResponse` - 握手消息
- `EventMessage` / `EventResponse` - 事件消息

**特点：**
- 所有插件都使用这些基础消息
- 定义了通信的基本结构
- `payload` 和 `data` 使用 `bytes` 类型，可以嵌套任意 Protobuf 消息

### storage/storage.proto - 存储插件协议

**用途：** 存储插件的业务消息定义

**包含：**
- 消息存储：`SaveMessageRequest` / `SaveMessageResponse`
- 离线消息：`SaveOfflineMessageRequest` / `PullOfflineMessagesRequest` 等
- 房间管理：`AddRoomMemberRequest` / `GetRoomMembersRequest` 等

**特点：**
- 类型安全的业务消息
- 清晰的字段定义和注释
- 编译时检查

## 添加新的插件协议

### 1. 创建新的 proto 文件

```bash
# 例如：网关插件
mkdir -p proto/gateway
touch proto/gateway/gateway.proto
```

### 2. 定义消息

```protobuf
// proto/gateway/gateway.proto
syntax = "proto3";

package v.plugin.gateway;

message HttpRequest {
  string method = 1;
  string path = 2;
  map<string, string> headers = 3;
  bytes body = 4;
}

message HttpResponse {
  int32 status_code = 1;
  map<string, string> headers = 2;
  bytes body = 3;
}
```

### 3. 更新 build.rs

```rust
let proto_files = vec![
    "proto/base.proto",
    "proto/storage/storage.proto",
    "proto/gateway/gateway.proto",  // 添加新文件
];
```

### 4. 更新 proto_codec.rs

```rust
pub mod pb {
    pub mod base {
        include!("proto/v.plugin.base.rs");
    }
    
    pub mod storage {
        include!("proto/v.plugin.storage.rs");
    }
    
    pub mod gateway {
        include!("proto/v.plugin.gateway.rs");
    }
    
    pub use base::*;
    pub use storage::*;
    pub use gateway::*;
}
```

### 5. 更新 protocol.rs

```rust
pub use crate::plugin::proto_codec::pb::{
    // 基础消息
    HandshakeRequest,
    HandshakeResponse,
    // 存储插件
    SaveMessageRequest,
    SaveMessageResponse,
    // 网关插件
    HttpRequest,
    HttpResponse,
};
```

## 使用示例

### 在插件中使用

```rust
use v::plugin::protocol::{
    SaveMessageRequest,
    SaveMessageResponse,
};

// 类型安全的请求处理
async fn handle_save_message(req: &SaveMessageRequest) -> Result<SaveMessageResponse> {
    // 字段访问有编译时检查
    let message_id = &req.message_id;
    let from_uid = &req.from_uid;
    
    // 保存消息...
    
    Ok(SaveMessageResponse {
        status: "ok".to_string(),
        message_id: message_id.clone(),
    })
}
```

### 事件分发

```rust
use prost::Message;

// 解码事件载荷
let event: EventMessage = decode_from_socket()?;

match event.event_type.as_str() {
    "storage.save_message" => {
        // 从 bytes 解码具体消息
        let req = SaveMessageRequest::decode(event.payload.as_slice())?;
        let resp = handle_save_message(&req).await?;
        
        // 编码响应
        let response = EventResponse {
            status: "ok".to_string(),
            flow: "continue".to_string(),
            data: resp.encode_to_vec(),
            error: String::new(),
        };
    }
    _ => {}
}
```

## 最佳实践

### 1. 命名规范

- **Package**: `v.plugin.<插件名>`
- **Message**: 使用 PascalCase，如 `SaveMessageRequest`
- **Field**: 使用 snake_case，如 `message_id`

### 2. 字段编号

- 1-15: 常用字段（单字节编码）
- 16-2047: 不常用字段
- 19000-19999: 保留字段

### 3. 注释

```protobuf
message SaveMessageRequest {
  string message_id = 1;    // 消息ID / Message ID
  string from_uid = 2;      // 发送者UID / Sender UID
  // 中英文双语注释
}
```

### 4. 版本兼容

- ✅ 添加新字段（向后兼容）
- ✅ 标记字段为 `reserved`
- ❌ 删除字段
- ❌ 修改字段类型
- ❌ 修改字段编号

### 5. 嵌套消息

```protobuf
message PullOfflineMessagesResponse {
  string status = 1;
  repeated OfflineMessage messages = 2;  // 嵌套消息
  int32 total = 3;
}

message OfflineMessage {
  string message_id = 1;
  string from_uid = 2;
  string content = 3;
  int64 timestamp = 4;
}
```

## 编译和生成

### 手动编译

```bash
cd v
cargo build
```

### 自动监听

build.rs 会自动监听 proto 文件变化并重新编译。

### 查看生成的代码

```bash
ls -la src/plugin/proto/
# v.plugin.base.rs       - 基础协议
# v.plugin.storage.rs    - 存储插件
```

## 故障排查

### 问题 1：编译错误

```
error: failed to compile proto files
```

**解决：**
- 检查 proto 语法
- 确保 package 名称正确
- 检查 import 路径

### 问题 2：找不到类型

```
error: cannot find type `SaveMessageRequest`
```

**解决：**
- 确保在 `proto_codec.rs` 中导出
- 确保在 `protocol.rs` 中重新导出
- 重新编译：`cargo clean && cargo build`

### 问题 3：字段不存在

```
error: no field `message_id` on type `SaveMessageRequest`
```

**解决：**
- 检查 proto 定义
- 重新生成代码：`touch proto/storage/storage.proto && cargo build`

## 参考资料

- [Protocol Buffers 官方文档](https://protobuf.dev/)
- [Prost 文档](https://docs.rs/prost/)
- [项目 Protobuf 指南](/PROTOBUF_FULL_REFACTOR.md)

---

**维护者**：VGO Team  
**最后更新**：2025-12-09
