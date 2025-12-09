# ✅ Proto 文件结构重组完成

## 新的目录结构

```
v/proto/
├── README.md                    # 详细使用文档
├── base.proto                   # 基础协议（握手、事件）
└── storage/                     # 存储插件协议
    └── storage.proto            # 存储相关消息定义
```

## 文件分类

### 1. base.proto - 基础协议

**包名：** `v.plugin.base`

**用途：** 所有插件通用的基础通信协议

**包含消息：**
- `HandshakeRequest` - 握手请求
- `HandshakeResponse` - 握手响应
- `EventMessage` - 事件消息
- `EventResponse` - 事件响应

**特点：**
- 所有插件都依赖这些基础消息
- `EventMessage.payload` 使用 `bytes` 类型，可以嵌套任意业务消息
- `EventResponse.data` 使用 `bytes` 类型，可以返回任意业务响应

### 2. storage/storage.proto - 存储插件协议

**包名：** `v.plugin.storage`

**用途：** 存储插件的业务消息定义

**包含消息：**

#### 消息存储
- `SaveMessageRequest` / `SaveMessageResponse`

#### 离线消息
- `SaveOfflineMessageRequest` / `SaveOfflineMessageResponse`
- `PullOfflineMessagesRequest` / `PullOfflineMessagesResponse`
- `AckOfflineMessagesRequest` / `AckOfflineMessagesResponse`
- `CountOfflineMessagesRequest` / `CountOfflineMessagesResponse`
- `OfflineMessage` - 离线消息结构

#### 房间管理
- `AddRoomMemberRequest` / `AddRoomMemberResponse`
- `RemoveRoomMemberRequest` / `RemoveRoomMemberResponse`
- `GetRoomMembersRequest` / `GetRoomMembersResponse`

## 代码生成

### build.rs 配置

```rust
let proto_files = vec![
    "proto/base.proto",              // 基础协议
    "proto/storage/storage.proto",   // 存储插件
];

prost_build::Config::new()
    .out_dir(&out_dir)
    .compile_protos(&proto_files, &["proto/"])?;
```

### 生成的文件

```
v/src/plugin/proto/
├── v.plugin.base.rs       # 基础协议生成的 Rust 代码
└── v.plugin.storage.rs    # 存储插件生成的 Rust 代码
```

## 代码组织

### proto_codec.rs - 模块导入

```rust
pub mod pb {
    // 基础协议
    pub mod base {
        include!("proto/v.plugin.base.rs");
    }
    
    // 存储插件协议
    pub mod storage {
        include!("proto/v.plugin.storage.rs");
    }
    
    // 重新导出常用类型
    pub use base::*;
    pub use storage::*;
}
```

### protocol.rs - 类型导出

```rust
// 重新导出 proto 生成的类型
pub use crate::plugin::proto_codec::pb::{
    // 基础消息
    HandshakeRequest,
    HandshakeResponse,
    EventMessage,
    EventResponse,
    // 存储插件消息
    SaveMessageRequest,
    SaveMessageResponse,
    PullOfflineMessagesRequest,
    PullOfflineMessagesResponse,
    // ... 其他消息
};
```

## 使用方式

### 1. 导入类型

```rust
use v::plugin::protocol::{
    EventMessage,
    EventResponse,
    SaveMessageRequest,
    SaveMessageResponse,
};
```

### 2. 创建消息

```rust
let request = SaveMessageRequest {
    message_id: "msg123".to_string(),
    from_uid: "user1".to_string(),
    to_uid: "user2".to_string(),
    content: "Hello".to_string(),
    timestamp: 1234567890,
    msg_type: "text".to_string(),
};
```

### 3. 编码/解码

```rust
use prost::Message;

// 编码
let bytes = request.encode_to_vec();

// 解码
let decoded = SaveMessageRequest::decode(bytes.as_slice())?;
```

### 4. 嵌套在事件中

```rust
// 创建事件消息
let event = EventMessage {
    event_type: "storage.save_message".to_string(),
    payload: request.encode_to_vec(),  // 嵌套业务消息
    timestamp: chrono::Utc::now().timestamp_millis(),
    trace_id: "trace123".to_string(),
};

// 发送事件...

// 接收端解码
let request = SaveMessageRequest::decode(event.payload.as_slice())?;
```

## 添加新插件协议

### 步骤 1：创建 proto 文件

```bash
mkdir -p v/proto/gateway
```

```protobuf
// v/proto/gateway/gateway.proto
syntax = "proto3";

package v.plugin.gateway;

message HttpRequest {
  string method = 1;
  string path = 2;
  bytes body = 3;
}

message HttpResponse {
  int32 status_code = 1;
  bytes body = 2;
}
```

### 步骤 2：更新 build.rs

```rust
let proto_files = vec![
    "proto/base.proto",
    "proto/storage/storage.proto",
    "proto/gateway/gateway.proto",  // 新增
];
```

### 步骤 3：更新 proto_codec.rs

```rust
pub mod pb {
    pub mod base {
        include!("proto/v.plugin.base.rs");
    }
    
    pub mod storage {
        include!("proto/v.plugin.storage.rs");
    }
    
    pub mod gateway {
        include!("proto/v.plugin.gateway.rs");  // 新增
    }
    
    pub use base::*;
    pub use storage::*;
    pub use gateway::*;  // 新增
}
```

### 步骤 4：更新 protocol.rs

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

### 步骤 5：编译

```bash
cargo build -p v
```

## 优势

### 1. 清晰的组织结构

- ✅ 基础协议和业务协议分离
- ✅ 每个插件有独立的 proto 文件
- ✅ 易于维护和扩展

### 2. 模块化

- ✅ 每个插件可以独立开发
- ✅ 不同插件的消息定义互不干扰
- ✅ 可以按需导入

### 3. 类型安全

- ✅ 编译时检查
- ✅ IDE 自动补全
- ✅ 重构支持

### 4. 性能

- ✅ 高效的二进制编码
- ✅ 零拷贝（某些场景）
- ✅ 小体积传输

## 最佳实践

### 1. 命名规范

```
proto/
├── base.proto                    # 基础协议
├── <插件名>/                     # 插件目录
│   └── <插件名>.proto            # 插件协议
```

### 2. Package 命名

```protobuf
// 基础协议
package v.plugin.base;

// 插件协议
package v.plugin.<插件名>;
```

### 3. 消息命名

```protobuf
// 请求：<动作><对象>Request
message SaveMessageRequest { }

// 响应：<动作><对象>Response
message SaveMessageResponse { }

// 数据结构：<对象>
message OfflineMessage { }
```

### 4. 字段注释

```protobuf
message SaveMessageRequest {
  string message_id = 1;    // 消息ID / Message ID
  string from_uid = 2;      // 发送者UID / Sender UID
  // 中英文双语注释
}
```

## 相关文档

- [Proto 使用文档](/v/proto/README.md)
- [Protobuf 完全重构方案](/PROTOBUF_FULL_REFACTOR.md)
- [Protobuf 仅支持说明](/PROTOBUF_ONLY.md)

---

**完成日期**：2025-12-09  
**维护者**：VGO Team
