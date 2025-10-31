# 性能监控和指标收集

本项目集成了全面的性能监控系统，可以实时收集和分析应用程序的性能指标。

## 功能特性

### 🔍 监控指标

- **请求统计**: 总请求数、成功请求数、失败请求数
- **响应时间**: 平均响应时间、最小/最大响应时间
- **吞吐量**: 每秒请求数 (QPS)
- **状态码分布**: 各种 HTTP 状态码的统计
- **路径统计**: 各个 API 端点的访问统计
- **系统资源**: 内存使用量、CPU 使用率

### 📊 API 端点

| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/metrics` | GET | 获取实时性能指标 |
| `/api/metrics/report` | GET | 获取详细性能报告 |
| `/api/metrics/reset` | POST | 重置所有性能指标 |
| `/health` | GET | 健康检查（包含性能信息） |

## 快速开始

### 1. 添加依赖

在 `Cargo.toml` 中添加必要的依赖：

```toml
[dependencies]
sys-info = "0.9"  # 系统信息获取
```

### 2. 集成中间件

```rust
use actix_web::{web, App, HttpServer};
use std::sync::Arc;
use vgo_rust::middleware::metrics::{MetricsMiddleware, PerformanceMonitor};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 创建性能监控器
    let performance_monitor = Arc::new(PerformanceMonitor::new());
    
    HttpServer::new(move || {
        App::new()
            // 添加性能监控中间件
            .wrap(MetricsMiddleware::new(performance_monitor.clone()))
            // 共享监控器数据
            .app_data(web::Data::new(performance_monitor.clone()))
            // 添加监控 API 路由
            .service(
                web::scope("/api")
                    .route("/metrics", web::get().to(get_metrics))
                    .route("/metrics/report", web::get().to(get_performance_report))
                    .route("/metrics/reset", web::post().to(reset_metrics))
            )
            .route("/health", web::get().to(health_check_with_metrics))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

### 3. 运行示例

```bash
# 运行性能监控示例
cargo run --example performance_monitoring

# 在另一个终端中测试
curl http://localhost:8080/api/metrics
curl http://localhost:8080/health
```

## API 响应格式

### 性能指标响应

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "total_requests": 1250,
    "successful_requests": 1200,
    "failed_requests": 50,
    "avg_response_time_ms": 125.5,
    "min_response_time_ms": 10.2,
    "max_response_time_ms": 2500.0,
    "requests_per_second": 45.2,
    "status_codes": {
      "200": 1150,
      "404": 30,
      "500": 20
    },
    "path_counts": {
      "/api/users": 500,
      "/api/orders": 300,
      "/health": 450
    },
    "memory_usage_bytes": 52428800,
    "cpu_usage_percent": 25.5,
    "uptime_seconds": 3600
  }
}
```

### 健康检查响应

```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "metrics": {
    "total_requests": 1250,
    "success_rate": 96.0,
    "avg_response_time_ms": 125.5,
    "requests_per_second": 45.2,
    "memory_usage_mb": 50.0,
    "cpu_usage_percent": 25.5
  }
}
```

## 健康状态判断

系统根据以下条件判断健康状态：

- ✅ **健康 (healthy)**: 
  - 平均响应时间 < 1000ms
  - CPU 使用率 < 80%
  - 成功率 > 95%

- ❌ **不健康 (unhealthy)**: 不满足上述任何一个条件

## 性能优化建议

### 1. 响应时间优化

- 当平均响应时间 > 500ms 时，考虑：
  - 优化数据库查询
  - 添加缓存层
  - 异步处理非关键任务

### 2. 内存使用优化

- 当内存使用 > 1GB 时，考虑：
  - 检查内存泄漏
  - 优化数据结构
  - 实现对象池

### 3. CPU 使用优化

- 当 CPU 使用率 > 70% 时，考虑：
  - 优化算法复杂度
  - 使用更高效的数据结构
  - 实现负载均衡

## 监控告警

可以基于性能指标设置告警规则：

```rust
// 示例：检查性能指标并发送告警
let metrics = monitor.get_metrics();

if metrics.avg_response_time_ms > 1000.0 {
    // 发送响应时间告警
    send_alert("高响应时间告警", &format!("平均响应时间: {}ms", metrics.avg_response_time_ms));
}

if metrics.cpu_usage_percent > 80.0 {
    // 发送 CPU 使用率告警
    send_alert("高CPU使用率告警", &format!("CPU使用率: {}%", metrics.cpu_usage_percent));
}

if metrics.total_requests > 0 {
    let error_rate = (metrics.failed_requests as f64 / metrics.total_requests as f64) * 100.0;
    if error_rate > 5.0 {
        // 发送错误率告警
        send_alert("高错误率告警", &format!("错误率: {:.2}%", error_rate));
    }
}
```

## 测试

运行性能监控相关测试：

```bash
# 运行所有测试
cargo test

# 运行性能监控测试
cargo test metrics

# 运行集成测试
cargo test --test integration_tests
```

## 配置选项

可以通过环境变量配置监控行为：

```bash
# 设置指标收集间隔（秒）
export METRICS_COLLECTION_INTERVAL=60

# 设置指标保留时间（小时）
export METRICS_RETENTION_HOURS=24

# 启用详细日志
export RUST_LOG=debug
```

## 生产环境建议

1. **定期重置指标**: 避免内存无限增长
2. **设置合理的收集间隔**: 平衡精度和性能
3. **监控磁盘空间**: 确保日志和指标存储充足
4. **配置告警阈值**: 根据业务需求调整
5. **定期备份指标数据**: 用于历史分析

## 故障排除

### 常见问题

1. **指标不更新**
   - 检查中间件是否正确注册
   - 验证监控器是否正确共享

2. **内存使用过高**
   - 定期调用 `reset_metrics()` 清理数据
   - 检查是否有内存泄漏

3. **CPU 使用率异常**
   - 检查系统信息获取频率
   - 优化指标计算逻辑

### 调试模式

启用调试日志查看详细信息：

```bash
RUST_LOG=vgo_rust::middleware::metrics=debug cargo run
```

## 扩展功能

### 自定义指标

```rust
// 添加自定义业务指标
impl PerformanceMonitor {
    pub fn record_custom_metric(&self, name: &str, value: f64) {
        // 实现自定义指标记录
    }
}
```

### 指标导出

```rust
// 导出指标到外部系统（如 Prometheus）
pub fn export_to_prometheus(&self) -> String {
    // 实现 Prometheus 格式导出
}
```

## 相关资源

- [Actix Web 中间件文档](https://actix.rs/docs/middleware/)
- [Rust 性能优化指南](https://nnethercote.github.io/perf-book/)
- [系统监控最佳实践](https://sre.google/sre-book/monitoring-distributed-systems/)