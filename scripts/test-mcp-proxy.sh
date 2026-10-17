#!/bin/bash
# Test MCP proxy setup tools

set -e

cd "$(dirname "$0")/.."

echo "🧪 Testing MCP Proxy Setup Tools"
echo "=================================="
echo ""

# Check if MCP server is available
if [ ! -f "target/release/np-mcp" ]; then
    echo "Building MCP server..."
    cargo build --release --bin np-mcp
fi

MCP="./target/release/np-mcp"

echo "1️⃣ Check current proxy status"
echo "------------------------------"
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"system_proxy_status","arguments":{}}}' | $MCP 2>/dev/null | jq '.result' || echo "Not available (normal if MCP server needs different invocation)"

echo ""
echo "2️⃣ Set system proxy to 127.0.0.1:8899"
echo "---------------------------------------"
echo "Note: This will modify your system proxy settings!"
echo "Press Enter to continue, or Ctrl+C to cancel..."
read

# Set proxy
networksetup -setwebproxy Wi-Fi 127.0.0.1 8899
networksetup -setsecurewebproxy Wi-Fi 127.0.0.1 8899
networksetup -setwebproxystate Wi-Fi on
networksetup -setsecurewebproxystate Wi-Fi on

echo "✅ Proxy configured!"

echo ""
echo "3️⃣ Verify proxy settings"
echo "-------------------------"
networksetup -getwebproxy Wi-Fi
echo ""
networksetup -getsecurewebproxy Wi-Fi

echo ""
echo "4️⃣ Test with curl"
echo "------------------"
echo "If NetProxy is running on :8899, this should work:"
curl -x http://127.0.0.1:8899 -s -o /dev/null -w "Status: %{http_code}\n" http://httpbin.org/get 2>/dev/null || echo "NetProxy not running or not responding"

echo ""
echo "Press Enter to clear proxy settings..."
read

echo ""
echo "5️⃣ Clear proxy settings"
echo "------------------------"
networksetup -setwebproxystate Wi-Fi off
networksetup -setsecurewebproxystate Wi-Fi off

echo "✅ Proxy cleared!"

echo ""
echo "6️⃣ Verify cleared"
echo "------------------"
networksetup -getwebproxy Wi-Fi

echo ""
echo "✅ Test complete!"
echo ""
echo "📝 To use via MCP:"
echo "  - system_proxy_set: Set proxy"
echo "  - system_proxy_clear: Clear proxy"
echo "  - system_proxy_status: Check status"
echo ""
echo "See docs/MCP-PROXY-SETUP.md for full documentation"
