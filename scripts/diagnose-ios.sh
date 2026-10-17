#!/bin/bash
# Diagnose iOS proxy connectivity issues

cd "$(dirname "$0")/.."

echo "🔍 NetProxy iOS Diagnostics"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Get Mac IP
MAC_IP=$(ifconfig en0 2>/dev/null | grep "inet " | grep -v 127.0.0.1 | awk '{print $2}')
if [ -z "$MAC_IP" ]; then
    MAC_IP=$(ifconfig | grep "inet " | grep -v 127.0.0.1 | head -1 | awk '{print $2}')
fi

echo "1️⃣ Network Configuration"
echo "   Mac IP Address: $MAC_IP"
echo "   Proxy Port: 8899"
echo ""

# Check if proxy is listening
echo "2️⃣ Proxy Status"
if lsof -i :8899 | grep -q LISTEN; then
    echo "   ✅ Proxy is LISTENING on port 8899"
    lsof -i :8899 | grep LISTEN | awk '{print "   " $1 " (PID: " $2 ")"}'

    # Check if listening on all interfaces
    if lsof -i :8899 | grep -q "\*:8899"; then
        echo "   ✅ Listening on ALL network interfaces (iOS can connect)"
    else
        echo "   ⚠️  Only listening on localhost (iOS CANNOT connect)"
        echo "   Fix: Restart with --listen 0.0.0.0:8899"
    fi
else
    echo "   ❌ Proxy is NOT listening"
    echo "   Fix: Launch NetProxy app and click 'Start Recording'"
fi
echo ""

# Check certificate
echo "3️⃣ CA Certificate"
if [ -f "./test-data/ca/ca.cert.pem" ]; then
    echo "   ✅ Certificate exists: ./test-data/ca/ca.cert.pem"

    # Check certificate details
    CERT_SUBJECT=$(openssl x509 -in ./test-data/ca/ca.cert.pem -noout -subject 2>/dev/null | sed 's/subject=//')
    echo "   Subject: $CERT_SUBJECT"

    # Copy to Desktop for easy transfer
    cp ./test-data/ca/ca.cert.pem ~/Desktop/NetProxy-CA.pem 2>/dev/null
    echo "   📋 Copied to: ~/Desktop/NetProxy-CA.pem"
else
    echo "   ⚠️  Certificate not found in ./test-data/ca/"

    # Try default location
    if [ -f ~/Library/Application\ Support/NetProxy/ca/ca.cert.pem ]; then
        echo "   ✅ Found at: ~/Library/Application Support/NetProxy/ca/"
        cp ~/Library/Application\ Support/NetProxy/ca/ca.cert.pem ~/Desktop/NetProxy-CA.pem
        echo "   📋 Copied to: ~/Desktop/NetProxy-CA.pem"
    else
        echo "   ❌ Certificate not found anywhere"
    fi
fi
echo ""

# Check firewall
echo "4️⃣ Firewall Status"
FW_STATUS=$(/usr/libexec/ApplicationFirewall/socketfilterfw --getglobalstate 2>/dev/null)
if echo "$FW_STATUS" | grep -q "State = 0"; then
    echo "   ✅ Firewall is disabled (good for testing)"
elif echo "$FW_STATUS" | grep -q "State = 1"; then
    echo "   ⚠️  Firewall is enabled (may block iOS connections)"
    echo "   Check if NetProxy is allowed in:"
    echo "   System Settings → Network → Firewall"
else
    echo "   ❓ Cannot determine firewall status"
fi
echo ""

# Check for captured flows
echo "5️⃣ Captured Traffic"
FLOW_COUNT=$(sqlite3 ./test-data/netproxy.db "SELECT COUNT(*) FROM flows;" 2>/dev/null || echo "0")
echo "   Total flows: $FLOW_COUNT"

if [ "$FLOW_COUNT" -gt 0 ]; then
    echo ""
    echo "   Recent hosts:"
    sqlite3 ./test-data/netproxy.db "SELECT DISTINCT host FROM flows ORDER BY flow DESC LIMIT 5;" 2>/dev/null | sed 's/^/   /'
fi
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 iOS Setup Checklist"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "On your iOS device:"
echo ""
echo "1. Install Certificate:"
echo "   • AirDrop ~/Desktop/NetProxy-CA.pem to iPhone"
echo "   • Tap file → Install → Enter passcode → Install → Done"
echo ""
echo "2. Trust Certificate:"
echo "   • Settings → General → About → Certificate Trust Settings"
echo "   • Enable 'NetProxy Local CA' toggle"
echo ""
echo "3. Configure Proxy:"
echo "   • Settings → Wi-Fi → (i) next to network"
echo "   • Configure Proxy → Manual"
echo "   • Server: $MAC_IP"
echo "   • Port: 8899"
echo "   • Save"
echo ""
echo "4. Test Connection:"
echo "   • Safari: http://$MAC_IP:8899"
echo "   • Should show error page (means proxy is reachable)"
echo ""
echo "5. Test Traffic:"
echo "   • Safari: https://httpbin.org/json"
echo "   • Check NetProxy app for new flows"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔧 Quick Fixes"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "If iOS shows 'Cannot connect to proxy':"
echo "  → Verify both devices on same Wi-Fi"
echo "  → Test: ping $MAC_IP (from another device)"
echo ""
echo "If 'This certificate is not trusted':"
echo "  → Go to Certificate Trust Settings and enable it"
echo "  → Restart iOS device"
echo ""
echo "If no traffic captured:"
echo "  → Make sure NetProxy app is running"
echo "  → Click 'Start Recording' (⌘R)"
echo "  → Check 'lsof -i :8899' shows LISTEN"
echo ""
echo "Full guide: docs/IOS-SETUP-GUIDE.md"
echo ""
