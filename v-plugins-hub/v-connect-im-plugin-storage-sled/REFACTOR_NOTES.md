# 存储插件重构说明
# Storage Plugin Refactoring Notes

## 🎯 重构目标 / Refactoring Goals

移除对 `Plugin` trait 和 `Context` 的依赖，改用专门的 `StorageEventListener` trait 和新的运行器。
Remove dependency on `Plugin` trait and `Context`, use dedicated `StorageEventListener` trait and new runner.

---

## ✅ 已完成的修改 / Completed Changes

### 1. **PDK 层面** (`v/src/plugin/pdk.rs`)

#### 新增函数 / New Function

```rust
pub async fn run_storage_server<L, C, F>(create_listener: F) -> Result<()>
where
    L: StorageEventListener + 'static,
    C: Default + DeserializeOwned,
    F: FnOnce(C) -> Result<L>,
```

**功能 / Features**:
- ✅ 专门为 `StorageEventListener` 设计
- ✅ 不需要实现 `Plugin` trait
- ✅ 不依赖 `Context`
- ✅ 直接使用 Protobuf 类型安全的请求/响应
- ✅ 自动事件分发到对应的 listener 方法

#### 新增包装器 / New Wrapper

```rust
struct StoragePluginWrapper {
    listener: Box<dyn StorageEventListener>,
    // ... 其他字段
}
```

**作用 / Purpose**:
- 将 `StorageEventListener` 适配到 `PluginHandler` 接口
- 在 `on_event()` 中调用 `dispatch_storage_event()`
- 自动处理 Protobuf 编解码

---

### 2. **插件层面** (`v-plugins-hub/v-connect-im-plugin-storage-sled/src/main.rs`)

#### 移除的代码 / Removed Code

```rust
// ❌ 不再需要
struct StoragePlugin {
    listener: SledStorageEventListener,
}

impl Plugin for StoragePlugin {
    type Config = SledStorageConfig;
    fn new() -> Self { ... }
    fn config(&self) -> Option<&Self::Config> { ... }
    fn config_mut(&mut self) -> Option<&mut Self::Config> { ... }
    fn on_config_update(&mut self, config: Self::Config) -> Result<()> { ... }
    fn receive(&mut self, ctx: &mut Context) -> Result<()> { ... }  // 死代码
}
```

#### 新的实现 / New Implementation

```rust
#[tokio::main]
async fn main() -> Result<()> {
    run_storage_server::<SledStorageEventListener, SledStorageConfig, _>(|config| {
        // 验证配置
        config.validate()?;
        
        // 创建监听器
        SledStorageEventListener::new(config)
    })
    .await
}
```

**优势 / Advantages**:
- ✅ **代码更简洁**: 从 ~110 行减少到 ~20 行
- ✅ **无死代码**: 移除了永远不会被调用的 `receive()` 方法
- ✅ **类型安全**: 直接使用 Protobuf 类型，无需 JSON 解析
- ✅ **更清晰**: 职责单一，只需实现 `StorageEventListener`

---

## 📊 对比 / Comparison

### 旧模式 / Old Pattern

```rust
// ❌ 需要实现 Plugin trait
impl Plugin for StoragePlugin {
    fn receive(&mut self, ctx: &mut Context) -> Result<()> {
        // 这个方法永远不会被调用！
        Ok(())
    }
}

// ✅ 实际工作的代码
impl StorageEventListener for SledStorageEventListener {
    async fn storage_message_save(&mut self, req: &SaveMessageRequest) 
        -> Result<SaveMessageResponse> {
        // 实际逻辑
    }
}
```

### 新模式 / New Pattern

```rust
// ✅ 只需实现 StorageEventListener
impl StorageEventListener for SledStorageEventListener {
    async fn storage_message_save(&mut self, req: &SaveMessageRequest) 
        -> Result<SaveMessageResponse> {
        // 实际逻辑
    }
}

// ✅ 使用专门的运行器
run_storage_server::<SledStorageEventListener, SledStorageConfig, _>(|config| {
    SledStorageEventListener::new(config)
})
```

---

## 🔄 事件处理流程 / Event Handling Flow

### 旧流程 / Old Flow

```
主服务 → Socket → PluginClient → PluginWrapper::on_event()
    ↓
创建 Context (JSON 解析)
    ↓
调用 Plugin::receive(ctx)  ← ❌ 死代码，永远不会执行
    ↓
(实际上直接跳到下一步)
    ↓
dispatch_storage_event() → StorageEventListener 方法
```

### 新流程 / New Flow

```
主服务 → Socket → PluginClient → StoragePluginWrapper::on_event()
    ↓
直接调用 dispatch_storage_event()
    ↓
自动解码 Protobuf
    ↓
调用 StorageEventListener 对应方法
    ↓
自动编码 Protobuf 响应
```

**改进 / Improvements**:
- ✅ 移除了无用的 Context 创建
- ✅ 移除了无用的 JSON 解析
- ✅ 直接使用 Protobuf，性能更好
- ✅ 流程更清晰，无死代码

---

## 📝 配置处理 / Configuration Handling

### 旧方式 / Old Way

```rust
impl Plugin for StoragePlugin {
    fn on_config_update(&mut self, config: Self::Config) -> Result<()> {
        // 需要手动重新创建 listener
        self.listener = SledStorageEventListener::new(config)?;
        Ok(())
    }
}
```

### 新方式 / New Way

```rust
// 配置在创建时传入，通过闭包验证
run_storage_server::<SledStorageEventListener, SledStorageConfig, _>(|config| {
    config.validate()?;  // 验证配置
    SledStorageEventListener::new(config)  // 创建监听器
})
```

**注意 / Note**: 
- 当前版本配置在启动时设置，不支持运行时热更新
- 如需热更新，需要在 `StoragePluginWrapper` 中实现 `config()` 方法

---

## 🎯 适用场景 / Use Cases

### 使用新模式 / Use New Pattern

✅ **存储插件** - 实现 `StorageEventListener`
✅ **认证插件** - 实现 `AuthEventListener`
✅ **其他专用插件** - 有明确的 EventListener trait

### 继续使用旧模式 / Continue Using Old Pattern

⚠️ **通用插件** - AI、过滤器等需要灵活处理各种事件
⚠️ **自定义事件** - 没有预定义的 EventListener trait

---

## 🚀 性能提升 / Performance Improvements

| 指标 | 旧模式 | 新模式 | 提升 |
|-----|-------|-------|------|
| **代码行数** | ~110 行 | ~20 行 | **-82%** |
| **JSON 解析** | 每次事件 | 无 | **100% 减少** |
| **类型检查** | 运行时 | 编译时 | **更安全** |
| **内存分配** | Context + JSON | 仅 Protobuf | **更少** |

---

## ✅ 验证 / Verification

```bash
# 编译检查
cargo check --package v-connect-im-plugin-storage-sled

# 构建插件
cargo build --release --package v-connect-im-plugin-storage-sled

# 运行插件
./target/release/v-connect-im-plugin-storage-sled --socket ./plugins/storage-sled.sock
```

---

## 📚 相关文件 / Related Files

| 文件 | 修改 | 说明 |
|-----|------|------|
| `v/src/plugin/pdk.rs` | ✅ 新增 | `run_storage_server()` 函数 |
| `v/src/plugin/pdk.rs` | ✅ 新增 | `StoragePluginWrapper` 结构 |
| `v-plugins-hub/.../src/main.rs` | ✅ 重构 | 移除 `Plugin` trait，使用新运行器 |
| `v-plugins-hub/.../src/sled_listener.rs` | ✅ 保持 | 实现 `StorageEventListener` |

---

## 🎉 总结 / Summary

### 主要改进 / Key Improvements

1. ✅ **移除死代码**: 删除了永远不会被调用的 `receive()` 方法
2. ✅ **简化架构**: 不再需要 `Plugin` trait 的包装层
3. ✅ **类型安全**: 直接使用 Protobuf 类型，编译时检查
4. ✅ **性能提升**: 移除 JSON 解析，减少内存分配
5. ✅ **代码更清晰**: 职责单一，易于理解和维护

### 向后兼容 / Backward Compatibility

- ✅ 不影响其他插件（AI、过滤器等）
- ✅ 不影响主服务
- ✅ `Context` 仍然保留，供通用插件使用

### 未来工作 / Future Work

- [ ] 为认证插件创建类似的 `run_auth_server()`
- [ ] 支持运行时配置热更新
- [ ] 添加更多 EventListener trait（网关、消息等）

---

**重构完成时间**: 2025-12-09
**Refactoring Completed**: 2025-12-09
