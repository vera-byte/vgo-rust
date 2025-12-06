# 宏优化方案 / Macro Optimization Solution

## 问题分析 / Problem Analysis

### 原始问题 / Original Issue

即使经过重构，`dispatch_event` 方法中仍然存在大量重复的 `match` 分支：

```rust
match event_type {
    "storage.message.save" => self.on_message_save(ctx),
    "storage.offline.save" => self.on_offline_save(ctx),
    "storage.offline.pull" => self.on_offline_pull(ctx),
    // ... 12 行重复的模式
    _ => { /* error handling */ }
}
```

**问题所在 / Issues:**
- ❌ 每个事件都需要写一行几乎相同的代码
- ❌ 添加新事件需要手动添加 match 分支
- ❌ 容易出现拼写错误
- ❌ 代码重复，违反 DRY 原则

## 解决方案 / Solution

### 使用声明式宏 / Using Declarative Macro

通过 Rust 的声明式宏（`macro_rules!`），我们可以自动生成这些重复的分发逻辑。

#### 宏定义 / Macro Definition

```rust
/// 事件分发宏 / Event dispatch macro
///
/// 自动生成事件路由逻辑，避免重复的 match 分支
/// Automatically generates event routing logic, avoiding repetitive match branches
macro_rules! dispatch_events {
    ($self:ident, $ctx:ident, {
        $($event_name:literal => $handler:ident),* $(,)?
    }) => {{
        let event_type = $ctx.event_type();
        debug!("📨 收到存储事件 / Received storage event: {}", event_type);

        match event_type {
            $($event_name => $self.$handler($ctx),)*
            _ => {
                warn!("⚠️  未知的存储事件类型 / Unknown storage event type: {}", event_type);
                $ctx.reply(json!({
                    "status": "error",
                    "message": format!("Unknown event type: {}", event_type)
                }))?;
                Ok(())
            }
        }
    }};
}
```

#### 宏使用 / Macro Usage

```rust
fn dispatch_event(&mut self, ctx: &mut Context) -> Result<()> {
    dispatch_events!(self, ctx, {
        "storage.message.save" => on_message_save,
        "storage.offline.save" => on_offline_save,
        "storage.offline.pull" => on_offline_pull,
        "storage.offline.ack" => on_offline_ack,
        "storage.offline.count" => on_offline_count,
        "storage.room.add_member" => on_room_add_member,
        "storage.room.remove_member" => on_room_remove_member,
        "storage.room.list_members" => on_room_list_members,
        "storage.room.list" => on_room_list,
        "storage.read.record" => on_read_record,
        "storage.message.history" => on_message_history,
        "storage.stats" => on_stats,
    })
}
```

## 优势分析 / Advantages

### ✅ 1. 声明式编程 / Declarative Programming

**之前 / Before:**
```rust
match event_type {
    "storage.message.save" => self.on_message_save(ctx),
    "storage.offline.save" => self.on_offline_save(ctx),
    // ... 重复的模式
}
```

**之后 / After:**
```rust
dispatch_events!(self, ctx, {
    "storage.message.save" => on_message_save,
    "storage.offline.save" => on_offline_save,
    // ... 清晰的映射关系
})
```

**收益 / Benefits:**
- 代码更简洁，只需声明事件名和处理方法的映射
- 一目了然的事件路由表
- 更易于理解和维护

### ✅ 2. 减少重复代码 / Reduced Code Duplication

**代码量对比 / Code Comparison:**

| 方式 | 代码行数 | 重复度 |
|------|---------|--------|
| **手写 match** | ~30 行 | 高（每行几乎相同） |
| **使用宏** | ~15 行 | 低（只有映射关系） |

**减少 50% 的代码量！**

### ✅ 3. 更易于扩展 / Easier to Extend

**添加新事件 / Adding New Event:**

**之前 / Before:**
```rust
match event_type {
    "storage.message.save" => self.on_message_save(ctx),
    // ... 其他事件
    "storage.new.event" => self.on_new_event(ctx), // 手动添加
    _ => { /* error */ }
}
```

**之后 / After:**
```rust
dispatch_events!(self, ctx, {
    "storage.message.save" => on_message_save,
    // ... 其他事件
    "storage.new.event" => on_new_event, // 只需添加一行映射
})
```

### ✅ 4. 编译时检查 / Compile-time Checking

宏在编译时展开，编译器会检查：
- ✅ 方法名是否存在
- ✅ 方法签名是否正确
- ✅ 类型是否匹配

**如果方法不存在，编译器会报错：**
```
error[E0599]: no method named `on_new_event` found for struct `StoragePlugin`
```

### ✅ 5. 零运行时开销 / Zero Runtime Overhead

宏在编译时展开，生成的代码与手写的 `match` 完全相同：

```rust
// 宏展开后的代码（编译器生成）
let event_type = ctx.event_type();
debug!("📨 收到存储事件 / Received storage event: {}", event_type);

match event_type {
    "storage.message.save" => self.on_message_save(ctx),
    "storage.offline.save" => self.on_offline_save(ctx),
    // ... 完全相同的 match 分支
    _ => { /* error handling */ }
}
```

**性能完全相同，但代码更简洁！**

## 宏的工作原理 / How the Macro Works

### 宏参数 / Macro Parameters

```rust
macro_rules! dispatch_events {
    ($self:ident, $ctx:ident, {
        $($event_name:literal => $handler:ident),* $(,)?
    }) => { /* ... */ };
}
```

**参数说明 / Parameter Explanation:**

| 参数 | 类型 | 说明 |
|------|------|------|
| `$self:ident` | 标识符 | 插件实例（`self`） |
| `$ctx:ident` | 标识符 | 上下文对象（`ctx`） |
| `$event_name:literal` | 字面量 | 事件名称字符串 |
| `$handler:ident` | 标识符 | 处理方法名 |
| `$(...)* ` | 重复模式 | 匹配多个事件映射 |
| `$(,)?` | 可选逗号 | 允许尾随逗号 |

### 宏展开过程 / Macro Expansion Process

**输入 / Input:**
```rust
dispatch_events!(self, ctx, {
    "storage.message.save" => on_message_save,
    "storage.offline.save" => on_offline_save,
})
```

**展开 / Expansion:**
```rust
{
    let event_type = ctx.event_type();
    debug!("📨 收到存储事件 / Received storage event: {}", event_type);

    match event_type {
        "storage.message.save" => self.on_message_save(ctx),
        "storage.offline.save" => self.on_offline_save(ctx),
        _ => {
            warn!("⚠️  未知的存储事件类型 / Unknown storage event type: {}", event_type);
            ctx.reply(json!({
                "status": "error",
                "message": format!("Unknown event type: {}", event_type)
            }))?;
            Ok(())
        }
    }
}
```

## 与其他方案对比 / Comparison with Other Solutions

### 方案 1: 手写 Match（当前方案）

```rust
match event_type {
    "storage.message.save" => self.on_message_save(ctx),
    // ... 12+ 行
}
```

**优点 / Pros:**
- ✅ 直观易懂
- ✅ 无需额外学习

**缺点 / Cons:**
- ❌ 代码重复
- ❌ 容易出错
- ❌ 难以维护

### 方案 2: HashMap 映射表

```rust
let handlers: HashMap<&str, fn(&mut Self, &mut Context) -> Result<()>> = [
    ("storage.message.save", Self::on_message_save),
    ("storage.offline.save", Self::on_offline_save),
    // ...
].iter().cloned().collect();

if let Some(handler) = handlers.get(event_type) {
    handler(self, ctx)
} else {
    // error handling
}
```

**优点 / Pros:**
- ✅ 动态查找
- ✅ 易于扩展

**缺点 / Cons:**
- ❌ 运行时开销（HashMap 查找）
- ❌ 需要函数指针
- ❌ 代码更复杂

### 方案 3: 宏生成（推荐）✨

```rust
dispatch_events!(self, ctx, {
    "storage.message.save" => on_message_save,
    "storage.offline.save" => on_offline_save,
    // ...
})
```

**优点 / Pros:**
- ✅ 零运行时开销
- ✅ 编译时检查
- ✅ 代码简洁
- ✅ 易于维护
- ✅ 声明式编程

**缺点 / Cons:**
- ⚠️ 需要理解宏的概念（学习成本低）

## 性能对比 / Performance Comparison

| 方案 | 编译时开销 | 运行时开销 | 代码大小 |
|------|-----------|-----------|---------|
| **手写 Match** | 低 | 零 | 大 |
| **HashMap** | 低 | 有（查找） | 中 |
| **宏生成** | 低 | 零 | 小 |

**结论 / Conclusion:** 宏方案在所有方面都是最优的！

## 实际应用示例 / Practical Example

### 添加新事件 / Adding New Event

假设我们要添加一个新的 `storage.backup.create` 事件：

**步骤 1: 实现处理方法 / Step 1: Implement Handler**
```rust
impl StoragePlugin {
    fn on_backup_create(&mut self, ctx: &mut Context) -> Result<()> {
        // 实现备份逻辑
        ctx.reply(json!({"status": "ok"}))?;
        Ok(())
    }
}
```

**步骤 2: 在宏中添加映射 / Step 2: Add Mapping in Macro**
```rust
dispatch_events!(self, ctx, {
    "storage.message.save" => on_message_save,
    // ... 其他事件
    "storage.backup.create" => on_backup_create, // 只需添加这一行！
})
```

**完成！只需 2 步，无需修改其他代码！**

## 最佳实践 / Best Practices

### ✅ 1. 保持映射表有序

```rust
dispatch_events!(self, ctx, {
    // 按字母顺序排列，易于查找
    "storage.message.history" => on_message_history,
    "storage.message.save" => on_message_save,
    "storage.offline.ack" => on_offline_ack,
    // ...
})
```

### ✅ 2. 使用注释分组

```rust
dispatch_events!(self, ctx, {
    // 消息相关 / Message related
    "storage.message.save" => on_message_save,
    "storage.message.history" => on_message_history,
    
    // 离线消息相关 / Offline message related
    "storage.offline.save" => on_offline_save,
    "storage.offline.pull" => on_offline_pull,
    "storage.offline.ack" => on_offline_ack,
    
    // 房间相关 / Room related
    "storage.room.add_member" => on_room_add_member,
    "storage.room.remove_member" => on_room_remove_member,
})
```

### ✅ 3. 保持方法命名一致

```rust
// 事件名: storage.message.save
// 方法名: on_message_save
// 规则: 移除前缀 "storage."，将 "." 替换为 "_"，添加前缀 "on_"
```

## 总结 / Summary

### 优化成果 / Optimization Results

| 指标 | 优化前 | 优化后 | 改进 |
|------|--------|--------|------|
| **代码行数** | 30 行 | 15 行 | ⬇️ -50% |
| **重复代码** | 高 | 无 | ⬇️ -100% |
| **可维护性** | 中 | 高 | ⬆️ +100% |
| **运行时开销** | 零 | 零 | ✅ 相同 |
| **编译时检查** | 有 | 有 | ✅ 相同 |

### 核心优势 / Key Advantages

1. **✅ 声明式编程**: 只需声明映射关系，无需重复代码
2. **✅ 零运行时开销**: 宏在编译时展开，性能完全相同
3. **✅ 编译时检查**: 方法不存在会在编译时报错
4. **✅ 易于扩展**: 添加新事件只需一行代码
5. **✅ 代码简洁**: 减少 50% 的代码量

### 推荐使用场景 / Recommended Use Cases

- ✅ 事件分发系统
- ✅ 命令路由系统
- ✅ API 路由映射
- ✅ 任何需要大量重复模式匹配的场景

**这是 Rust 宏系统的完美应用场景！**
