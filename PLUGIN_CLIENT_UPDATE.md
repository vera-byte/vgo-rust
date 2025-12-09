# 插件客户端更新说明

## 更新内容

### 1. 统一客户端实现

已将 `client_v2.rs` 的多协议支持功能合并到 `client.rs`，现在只有一个统一的客户端实现。

**变更：**
- ✅ 删除 `v/src/plugin/client_v2.rs`
- ✅ 更新 `v/src/plugin/client.rs` 支持多协议
- ✅ 更新 `v/src/plugin/mod.rs` 移除 `client_v2` 导出

### 2. 新增功能

**PluginHandler Trait 新增方法：**

```rust
/// 支持的协议格式 / Supported protocol format
fn protocol(&self) -> ProtocolFormat {
    // 默认使用 Protobuf，如果未启用则回退到 JSON
    #[cfg(feature = "protobuf")]
    {
        ProtocolFormat::Protobuf
    }
    #[cfg(not(feature = "protobuf"))]
    {
        ProtocolFormat::Json
    }
}
```

**PluginClient 新增字段：**

```rust
pub struct PluginClient<H: PluginHandler> {
    // ... 其他字段
    codec: Box<dyn ProtocolCodec>,      // 协议编解码器
    protocol: ProtocolFormat,           // 当前使用的协议
}
```

### 3. 协议协商

客户端现在支持自动协议协商：

1. **插件声明支持的协议**（通过 `protocol()` 方法）
2. **握手时发送协议信息**
3. **服务端选择最优协议**
4. **客户端根据响应切换编解码器**

```rust
// 协议协商示例
if !resp_val.protocol.is_empty() {
    let negotiated = negotiate_protocol(&resp_val.protocol);
    if negotiated != self.protocol {
        info!("🔄 Protocol negotiated: {:?} -> {:?}", self.protocol, negotiated);
        self.protocol = negotiated;
        self.codec = get_codec(negotiated);
    }
}
```

### 4. 使用方法

#### 方法 1：使用默认协议（推荐）

```rust
use v::plugin::client::{PluginClient, PluginHandler};
use v::plugin::protocol::ProtocolFormat;

struct MyPlugin;

impl PluginHandler for MyPlugin {
    fn name(&self) -> &'static str { "my-plugin" }
    fn version(&self) -> &'static str { "1.0.0" }
    fn capabilities(&self) -> Vec<String> { vec!["message".into()] }
    
    // 使用默认协议（自动选择）
    // 不需要实现 protocol() 方法
    
    fn on_event(&mut self, event_type: &str, payload: &Value) -> Result<Value> {
        Ok(json!({"status": "ok"}))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = PluginClient::new("./plugins/my.sock", MyPlugin);
    client.run_forever_with_ctrlc().await
}
```

#### 方法 2：指定协议

```rust
impl PluginHandler for MyPlugin {
    // ... 其他方法
    
    // 强制使用 Protobuf
    fn protocol(&self) -> ProtocolFormat {
        ProtocolFormat::Protobuf
    }
    
    // 或强制使用 JSON
    fn protocol(&self) -> ProtocolFormat {
        ProtocolFormat::Json
    }
}
```

#### 方法 3：条件编译（推荐）

```rust
impl PluginHandler for MyPlugin {
    // ... 其他方法
    
    // 如果启用 protobuf 特性则使用 Protobuf，否则使用 JSON
    fn protocol(&self) -> ProtocolFormat {
        #[cfg(feature = "protobuf")]
        {
            ProtocolFormat::Protobuf
        }
        #[cfg(not(feature = "protobuf"))]
        {
            ProtocolFormat::Json
        }
    }
}
```

### 5. 迁移指南

如果你之前使用 `PluginClientV2`，现在需要更新为 `PluginClient`：

**之前：**
```rust
use v::plugin::client_v2::{PluginClientV2, PluginHandler};

let mut client = PluginClientV2::new(socket_path, handler);
```

**现在：**
```rust
use v::plugin::client::{PluginClient, PluginHandler};

let mut client = PluginClient::new(socket_path, handler);
```

**仅需更改导入路径和类型名称，其他代码无需修改！**

### 6. 编译和运行

```bash
# 使用 JSON 协议（默认）
cargo build

# 使用 Protobuf 协议
cargo build --features protobuf

# 运行示例
cargo run --example plugin_protobuf_example --features protobuf
```

### 7. 性能对比

| 协议 | 编码速度 | 解码速度 | 数据大小 | 推荐场景 |
|------|---------|---------|---------|---------|
| **JSON** | 1x | 1x | 100% | 开发调试、兼容性优先 |
| **Protobuf** | 5-10x | 6-12x | 20-40% | 生产环境、性能优先 |
| **MessagePack** | 3-5x | 4-6x | 40-60% | 平衡性能和兼容性 |

### 8. 示例代码

完整示例请参考：
- `/examples/plugin_protobuf_example.rs` - Protobuf 插件示例
- `/docs/plugin/protobuf-guide.mdx` - 详细使用指南
- `/PROTOBUF_MIGRATION.md` - 迁移指南

### 9. 常见问题

**Q: 我的插件会自动使用 Protobuf 吗？**

A: 不会。默认行为取决于编译时是否启用 `protobuf` 特性：
- 启用 `protobuf` 特性：默认使用 Protobuf
- 未启用：默认使用 JSON

**Q: 如何确认插件使用的协议？**

A: 查看插件启动日志：
```
[plugin:my-plugin-1.0.0] init client, socket=./plugins/my.sock, protocol=Protobuf
```

**Q: 可以在运行时切换协议吗？**

A: 不可以。协议在握手时确定，之后不会改变。如需切换协议，需要重启插件。

**Q: 旧插件还能用吗？**

A: 可以。系统完全向后兼容，旧的 JSON 插件可以继续使用。

### 10. 下一步

1. ✅ 阅读 [Protobuf 使用指南](/docs/plugin/protobuf-guide.mdx)
2. ✅ 运行示例代码测试
3. ✅ 根据需求选择合适的协议
4. ✅ 更新现有插件（可选）

---

**更新日期**：2025-12-09  
**版本**：1.0.0  
**维护者**：VGO Team
