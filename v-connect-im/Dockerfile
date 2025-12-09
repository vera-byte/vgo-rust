# v-connect-im Dockerfile
# 多阶段构建，优化镜像大小 / Multi-stage build for optimized image size

# 构建阶段 / Build stage
FROM rust:1.75-slim as builder

# 安装构建依赖 / Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录 / Set working directory
WORKDIR /build

# 复制整个工作空间 / Copy entire workspace
COPY . .

# 构建 release 版本 / Build release version
RUN cargo build --release --package v-connect-im

# 构建插件 / Build plugins
RUN cargo build --release --package v-connect-im-plugin-storage-sled && \
    cargo build --release --package v-connect-im-plugin-gateway

# 运行阶段 / Runtime stage
FROM debian:bookworm-slim

# 安装运行时依赖 / Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# 创建非 root 用户 / Create non-root user
RUN useradd -m -u 1000 -s /bin/bash vconnect

# 设置工作目录 / Set working directory
WORKDIR /app

# 创建必要的目录 / Create necessary directories
RUN mkdir -p /app/{bin,config,logs,plugins/sockets,data} && \
    chown -R vconnect:vconnect /app

# 从构建阶段复制二进制文件 / Copy binaries from build stage
COPY --from=builder /build/target/release/v-connect-im /app/bin/
COPY --from=builder /build/target/release/v-connect-im-plugin-storage-sled /app/plugins/
COPY --from=builder /build/target/release/v-connect-im-plugin-gateway /app/plugins/

# 复制配置文件 / Copy configuration files
COPY --from=builder /build/v-connect-im/config/default.toml /app/config/

# 复制插件配置 / Copy plugin configs
COPY --from=builder /build/v-plugins-hub/v-connect-im-plugin-storage-sled/plugin.json /app/plugins/storage-sled.json
COPY --from=builder /build/v-plugins-hub/v-connect-im-plugin-gateway/plugin.json /app/plugins/gateway.json

# 设置权限 / Set permissions
RUN chmod +x /app/bin/v-connect-im && \
    chmod +x /app/plugins/v-connect-im-plugin-storage-sled && \
    chmod +x /app/plugins/v-connect-im-plugin-gateway

# 切换到非 root 用户 / Switch to non-root user
USER vconnect

# 暴露端口 / Expose ports
EXPOSE 8080 8081

# 设置环境变量 / Set environment variables
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# 健康检查 / Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# 启动命令 / Start command
CMD ["/app/bin/v-connect-im"]
