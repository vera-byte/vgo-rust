#!/bin/bash
# 插件间通信功能测试脚本 / Plugin inter-communication test script

set -e

echo "🧪 插件间通信功能测试 / Plugin Inter-Communication Tests"
echo "============================================================"

# 颜色定义 / Color definitions
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# 服务器地址 / Server address
SERVER_URL="${SERVER_URL:-http://localhost:8080}"

# 测试计数器 / Test counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# 测试函数 / Test function
test_case() {
    local test_name="$1"
    local test_command="$2"
    local expected_status="${3:-200}"
    
    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    echo ""
    echo "📝 测试 $TOTAL_TESTS: $test_name"
    echo "   Test $TOTAL_TESTS: $test_name"
    
    # 执行测试 / Execute test
    response=$(eval "$test_command" 2>&1)
    status=$?
    
    if [ $status -eq 0 ]; then
        echo -e "${GREEN}✅ 通过 / PASSED${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        echo "   响应 / Response: $response" | head -n 5
    else
        echo -e "${RED}❌ 失败 / FAILED${NC}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        echo "   错误 / Error: $response"
    fi
}

echo ""
echo "🔍 检查服务器状态 / Checking server status..."

# 检查服务器是否运行 / Check if server is running
if ! curl -s "$SERVER_URL/health" > /dev/null 2>&1; then
    echo -e "${YELLOW}⚠️  警告：服务器未运行 / WARNING: Server is not running${NC}"
    echo "   请先启动服务器 / Please start the server first:"
    echo "   cargo run -- --config config/default.toml"
    echo ""
    echo "   以下测试将展示 API 调用示例（不会实际执行）"
    echo "   The following tests will show API call examples (not actually executed)"
    DEMO_MODE=1
else
    echo -e "${GREEN}✅ 服务器正在运行 / Server is running${NC}"
    DEMO_MODE=0
fi

echo ""
echo "=" | tr -d '\n' | head -c 60
echo ""

# ==================== 测试 1: 插件 RPC 调用 ====================
echo ""
echo "📞 测试组 1: 插件 RPC 调用 / Test Group 1: Plugin RPC Call"
echo "-" | tr -d '\n' | head -c 60
echo ""

if [ $DEMO_MODE -eq 0 ]; then
    test_case "RPC 调用 - 正常场景" \
        "curl -s -X POST $SERVER_URL/v1/plugins/inter-communication \
        -H 'Content-Type: application/json' \
        -d '{
            \"from_plugin\": \"example\",
            \"to_plugin\": \"storage-sled\",
            \"method\": \"get_stats\",
            \"params\": {}
        }'"
    
    test_case "RPC 调用 - 目标插件不存在" \
        "curl -s -X POST $SERVER_URL/v1/plugins/inter-communication \
        -H 'Content-Type: application/json' \
        -d '{
            \"from_plugin\": \"example\",
            \"to_plugin\": \"non-existent\",
            \"method\": \"test\",
            \"params\": {}
        }'"
else
    echo "示例 API 调用 / Example API Call:"
    echo "curl -X POST $SERVER_URL/v1/plugins/inter-communication \\"
    echo "  -H 'Content-Type: application/json' \\"
    echo "  -d '{"
    echo "    \"from_plugin\": \"example\","
    echo "    \"to_plugin\": \"storage-sled\","
    echo "    \"method\": \"get_stats\","
    echo "    \"params\": {}"
    echo "  }'"
fi

# ==================== 测试 2: 点对点消息 ====================
echo ""
echo "💌 测试组 2: 点对点消息 / Test Group 2: P2P Messaging"
echo "-" | tr -d '\n' | head -c 60
echo ""

if [ $DEMO_MODE -eq 0 ]; then
    test_case "P2P 消息 - 正常发送" \
        "curl -s -X PUT $SERVER_URL/v1/plugins/inter-communication \
        -H 'Content-Type: application/json' \
        -d '{
            \"from_plugin\": \"example\",
            \"to_plugin\": \"storage-sled\",
            \"message\": {
                \"type\": \"notification\",
                \"content\": \"test message\"
            }
        }'"
else
    echo "示例 API 调用 / Example API Call:"
    echo "curl -X PUT $SERVER_URL/v1/plugins/inter-communication \\"
    echo "  -H 'Content-Type: application/json' \\"
    echo "  -d '{"
    echo "    \"from_plugin\": \"example\","
    echo "    \"to_plugin\": \"storage-sled\","
    echo "    \"message\": {"
    echo "      \"type\": \"notification\","
    echo "      \"content\": \"test message\""
    echo "    }"
    echo "  }'"
fi

# ==================== 测试 3: 广播消息 ====================
echo ""
echo "📢 测试组 3: 广播消息 / Test Group 3: Broadcast"
echo "-" | tr -d '\n' | head -c 60
echo ""

if [ $DEMO_MODE -eq 0 ]; then
    test_case "广播 - 无过滤" \
        "curl -s -X PATCH $SERVER_URL/v1/plugins/inter-communication \
        -H 'Content-Type: application/json' \
        -d '{
            \"from_plugin\": \"example\",
            \"message\": {
                \"event\": \"test_broadcast\"
            }
        }'"
    
    test_case "广播 - 能力过滤" \
        "curl -s -X PATCH $SERVER_URL/v1/plugins/inter-communication \
        -H 'Content-Type: application/json' \
        -d '{
            \"from_plugin\": \"example\",
            \"message\": {
                \"event\": \"storage_sync\"
            },
            \"filter_capabilities\": [\"storage\"]
        }'"
else
    echo "示例 API 调用 / Example API Call:"
    echo "curl -X PATCH $SERVER_URL/v1/plugins/inter-communication \\"
    echo "  -H 'Content-Type: application/json' \\"
    echo "  -d '{"
    echo "    \"from_plugin\": \"example\","
    echo "    \"message\": {"
    echo "      \"event\": \"test_broadcast\""
    echo "    },"
    echo "    \"filter_capabilities\": [\"storage\"]"
    echo "  }'"
fi

# ==================== 测试总结 ====================
echo ""
echo "=" | tr -d '\n' | head -c 60
echo ""
echo "📊 测试总结 / Test Summary"
echo "=" | tr -d '\n' | head -c 60
echo ""

if [ $DEMO_MODE -eq 0 ]; then
    echo "总测试数 / Total Tests: $TOTAL_TESTS"
    echo -e "${GREEN}通过 / Passed: $PASSED_TESTS${NC}"
    if [ $FAILED_TESTS -gt 0 ]; then
        echo -e "${RED}失败 / Failed: $FAILED_TESTS${NC}"
    else
        echo "失败 / Failed: $FAILED_TESTS"
    fi
    
    if [ $FAILED_TESTS -eq 0 ]; then
        echo ""
        echo -e "${GREEN}🎉 所有测试通过！ / All tests passed!${NC}"
        exit 0
    else
        echo ""
        echo -e "${RED}⚠️  部分测试失败 / Some tests failed${NC}"
        exit 1
    fi
else
    echo "演示模式 - 未执行实际测试"
    echo "Demo mode - No actual tests executed"
    echo ""
    echo "要运行实际测试，请先启动服务器："
    echo "To run actual tests, please start the server first:"
    echo "  1. cargo run -- --config config/default.toml"
    echo "  2. 启动测试插件 / Start test plugins"
    echo "  3. 重新运行此脚本 / Re-run this script"
fi

echo ""
echo "📚 更多信息 / More Information:"
echo "   - 测试指南 / Test Guide: docs/plugin_communication_test_guide.md"
echo "   - 功能文档 / Documentation: docs/plugin_inter_communication.md"
echo "   - 演示程序 / Demo: cargo run --example plugin_communication_demo"
