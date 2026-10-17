# iOS Device Proxy Setup Guide

Complete guide to capture iOS traffic with NetProxy.

## Prerequisites

✅ Proxy listening on: `*:8899` (all interfaces)  
✅ Your Mac IP: **192.168.1.2**  
✅ Firewall: Disabled  

---

## Step 1: Start NetProxy Engine

**CRITICAL:** You must start the proxy engine first!

```bash
# Terminal 1: Make sure daemon is running
cd /Users/smkwinner/Desktop/workspace/netproxy
./scripts/start-fresh.sh

# Terminal 2: Launch SwiftUI app
cd app
swift run

# In the app:
# Click "Start Recording" (⌘R)
# Wait for status to show "Recording"
```

**Verify engine is running:**
```bash
# Should show traffic on port 8899
lsof -i :8899 | grep LISTEN
```

---

## Step 2: Export CA Certificate

The iOS device needs to trust NetProxy's certificate.

**Get the certificate:**
```bash
cd /Users/smkwinner/Desktop/workspace/netproxy

# Certificate is here:
ls -la ./test-data/ca/ca.cert.pem

# Copy to Desktop for easy transfer
cp ./test-data/ca/ca.cert.pem ~/Desktop/NetProxy-CA.pem

# Or if using default location:
cp ~/Library/Application\ Support/NetProxy/ca/ca.cert.pem ~/Desktop/NetProxy-CA.pem 2>/dev/null || echo "Using test-data location"

echo "Certificate copied to ~/Desktop/NetProxy-CA.pem"
```

---

## Step 3: Install Certificate on iOS

### Method 1: AirDrop (Easiest)

1. **On Mac:**
   - Find `NetProxy-CA.pem` on Desktop
   - Right-click → Share → AirDrop
   - Select your iPhone/iPad

2. **On iOS:**
   - Tap the AirDrop notification
   - Tap "Allow"
   - A profile appears → Tap "Install"
   - Enter passcode
   - Tap "Install" again (warning appears - this is normal)
   - Tap "Done"

### Method 2: Email

1. Email `NetProxy-CA.pem` to yourself
2. Open email on iOS
3. Tap the attachment
4. Follow install steps above

### Method 3: Web Server

```bash
# Serve certificate via HTTP
cd ~/Desktop
python3 -m http.server 8000

# On iOS Safari:
# Visit: http://192.168.1.2:8000
# Tap NetProxy-CA.pem
# Follow install steps
```

---

## Step 4: Trust the Certificate

**Installing ≠ Trusting!** iOS requires two steps:

1. **Go to Settings → General → VPN & Device Management**
   - You should see "NetProxy Local CA" profile
   - If not, certificate didn't install properly

2. **Go to Settings → General → About → Certificate Trust Settings**
   - Scroll down to "Enable Full Trust for Root Certificates"
   - Toggle ON "NetProxy Local CA"
   - Tap "Continue" on warning

**Without this step, HTTPS traffic won't work!**

---

## Step 5: Configure Proxy on iOS

1. **Settings → Wi-Fi**
2. Tap the **(i)** icon next to your connected network
3. Scroll down to **HTTP Proxy**
4. Tap **Configure Proxy**
5. Select **Manual**
6. Enter:
   - **Server:** `192.168.1.2` (your Mac's IP)
   - **Port:** `8899`
   - **Authentication:** OFF
7. Tap **Save**

**Verify:**
- The Wi-Fi network should now show "HTTP Proxy" with a checkmark
- Blue dot next to the network name

---

## Step 6: Test Traffic Capture

### Test with Safari on iOS:

1. Open Safari
2. Visit: `http://httpbin.org/json`
3. You should see JSON data

### Check NetProxy:

**On Mac:**
```bash
# Check if flows were captured
cd /Users/smkwinner/Desktop/workspace/netproxy
sqlite3 ./test-data/netproxy.db "SELECT host, path, method FROM flows WHERE host LIKE '%httpbin%';"
```

**In NetProxy app:**
- You should see new flows appearing
- Host: `httpbin.org`
- Method: `GET`
- Path: `/json`

---

## Troubleshooting

### ❌ No Traffic Captured

**1. Is the engine running?**
```bash
lsof -i :8899
# Should show: netproxyd ... LISTEN
```

If not: Launch SwiftUI app and click "Start Recording"

**2. Can iOS reach the Mac?**

On iOS Safari, visit: `http://192.168.1.2:8899`

- **Expected:** NetProxy error page or blank page
- **If timeout:** Network issue - check:
  - Both devices on same Wi-Fi
  - Mac's IP is correct (`ifconfig en0 | grep inet`)
  - No VPN on either device

**3. Is proxy configured on iOS?**

Settings → Wi-Fi → (i) → HTTP Proxy

- Should show: Manual, Server: 192.168.1.2, Port: 8899
- If not: Reconfigure

**4. Is certificate trusted?**

Settings → General → About → Certificate Trust Settings

- "NetProxy Local CA" should be **ON**
- If certificate missing: Reinstall from Step 3

### ❌ HTTP Works, HTTPS Doesn't

**Symptom:** Can visit `http://` sites but `https://` times out or shows certificate error

**Fix:** Certificate not trusted properly

1. Go to: Settings → General → About → Certificate Trust Settings
2. Enable "NetProxy Local CA"
3. **Restart iOS device** (sometimes required)

### ❌ Some Apps Don't Work

**Normal:** Many apps use certificate pinning and won't work through MITM proxy:
- Banking apps
- Some social media apps
- Apps with high security

**These should work:**
- Safari
- Chrome/Firefox
- Most HTTP clients
- Apps without pinning

---

## Testing Checklist

- [ ] NetProxy daemon running
- [ ] SwiftUI app shows "Recording"
- [ ] Port 8899 listening (verified with `lsof`)
- [ ] Certificate exported
- [ ] Certificate installed on iOS (in VPN & Device Management)
- [ ] Certificate **trusted** on iOS (in Certificate Trust Settings)
- [ ] Proxy configured on iOS (Manual, 192.168.1.2:8899)
- [ ] iOS can reach Mac (test: `http://192.168.1.2:8899` in Safari)
- [ ] HTTP test works (`http://httpbin.org/json`)
- [ ] HTTPS test works (`https://httpbin.org/json`)
- [ ] Flows appear in NetProxy app

---

## Common Issues

### "Safari Cannot Open the Page"

**Cause:** iOS cannot reach the proxy

**Fix:**
1. Verify Mac IP: `ifconfig en0 | grep inet`
2. Update proxy settings on iOS with correct IP
3. Test connectivity: ping Mac from another device

### "This Certificate Is Not Trusted"

**Cause:** Certificate installed but not trusted

**Fix:**
1. Settings → General → About → Certificate Trust Settings
2. Toggle ON "NetProxy Local CA"
3. Restart iOS device

### "The Network Connection Was Lost"

**Cause:** Engine not running

**Fix:**
1. Check `lsof -i :8899` shows LISTEN
2. If not: Launch app and click "Start Recording"

### Proxy Settings Keep Disappearing

**Cause:** iOS resets proxy on network change

**Fix:**
- Reconfigure proxy each time you reconnect to Wi-Fi
- Or use a Profile (Configuration Profile) to persist settings

---

## Quick Reference

**Mac IP:** 192.168.1.2  
**Proxy Port:** 8899  
**Certificate:** `~/Desktop/NetProxy-CA.pem`

**iOS Proxy Settings:**
```
Manual Proxy
Server: 192.168.1.2
Port: 8899
Authentication: Off
```

**Verify Capture:**
```bash
sqlite3 ./test-data/netproxy.db "SELECT COUNT(*) FROM flows;"
```

---

## Advanced: Persistent iOS Proxy Profile

Create a Configuration Profile to avoid manual setup:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>PayloadContent</key>
    <array>
        <dict>
            <key>PayloadType</key>
            <string>com.apple.proxy.http.global</string>
            <key>PayloadVersion</key>
            <integer>1</integer>
            <key>PayloadIdentifier</key>
            <string>com.netproxy.proxy</string>
            <key>PayloadUUID</key>
            <string>INSERT-UUID-HERE</string>
            <key>PayloadDisplayName</key>
            <string>NetProxy HTTP Proxy</string>
            <key>ProxyServer</key>
            <string>192.168.1.2</string>
            <key>ProxyServerPort</key>
            <integer>8899</integer>
        </dict>
    </array>
    <key>PayloadDisplayName</key>
    <string>NetProxy Proxy Settings</string>
    <key>PayloadIdentifier</key>
    <string>com.netproxy</string>
    <key>PayloadType</key>
    <string>Configuration</string>
    <key>PayloadUUID</key>
    <string>INSERT-UUID-HERE</string>
    <key>PayloadVersion</key>
    <integer>1</integer>
</dict>
</plist>
```

Save as `NetProxy.mobileconfig` and AirDrop to iOS.

---

**Need help?** Check `netproxy-daemon.log` for errors.
