#!/bin/bash

# QUIC 测试证书生成脚本
# 用于快速生成自签名证书用于测试

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CERT_DIR="$SCRIPT_DIR/../certs"

echo "🔐 生成 QUIC 测试证书..."
echo ""

# 创建 certs 目录
mkdir -p "$CERT_DIR"

# 生成私钥
echo "📝 生成私钥..."
openssl genrsa -out "$CERT_DIR/server.key" 2048

# 生成证书签名请求
echo "📝 生成证书签名请求..."
openssl req -new -key "$CERT_DIR/server.key" -out "$CERT_DIR/server.csr" \
  -subj "/C=CN/ST=State/L=City/O=v-connect-im/CN=localhost"

# 生成自签名证书（有效期365天）
echo "📝 生成自签名证书..."
openssl x509 -req -days 365 -in "$CERT_DIR/server.csr" \
  -signkey "$CERT_DIR/server.key" -out "$CERT_DIR/server.crt"

# 清理临时文件
rm -f "$CERT_DIR/server.csr"

echo ""
echo "✅ 证书生成完成！"
echo ""
echo "证书文件："
echo "  - 私钥: $CERT_DIR/server.key"
echo "  - 证书: $CERT_DIR/server.crt"
echo ""
echo "⚠️  注意：这是自签名证书，仅用于测试。生产环境请使用 CA 签发的正式证书。"
echo ""

