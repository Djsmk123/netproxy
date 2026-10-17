# MCP Tools for macOS Proxy Setup

NetProxy MCP server includes tools to automatically configure macOS system proxy settings!

---

## Available Tools

### 1. `system_proxy_set`

**Configure macOS system-wide proxy (HTTP + HTTPS)**

```json
{
  "name": "system_proxy_set",
  "arguments": {
    "host": "127.0.0.1",
    "port": 8899,
    "interface": "Wi-Fi"  // Optional, defaults to primary interface
  }
}
```

**What it does:**
- Sets HTTP proxy (port 80 traffic)
- Sets HTTPS proxy (port 443 traffic)
- Enables both proxies
- Works on Wi-Fi, Ethernet, or any network interface

**Response:**
```json
{
  "success": true,
  "interface": "Wi-Fi",
  "http_proxy": "127.0.0.1:8899",
  "https_proxy": "127.0.0.1:8899"
}
```

---

### 2. `system_proxy_clear`

**Remove all proxy settings**

```json
{
  "name": "system_proxy_clear",
  "arguments": {
    "interface": "Wi-Fi"  // Optional
  }
}
```

**What it does:**
- Disables HTTP proxy
- Disables HTTPS proxy
- Restores direct internet connection

**Response:**
```json
{
  "success": true,
  "interface": "Wi-Fi",
  "message": "Proxy settings cleared"
}
```

---

### 3. `system_proxy_status`

**Check current proxy configuration**

```json
{
  "name": "system_proxy_status",
  "arguments": {
    "interface": "Wi-Fi"  // Optional
  }
}
```

**Response:**
```json
{
  "interface": "Wi-Fi",
  "http": {
    "enabled": true,
    "server": "127.0.0.1",
    "port": 8899
  },
  "https": {
    "enabled": true,
    "server": "127.0.0.1",
    "port": 8899
  }
}
```

---

## Usage Examples

### Example 1: AI-Assisted Setup

**User:** "Set up NetProxy on my Mac"

**AI via MCP:**
```json
// Step 1: Start NetProxy engine
{
  "name": "engine_start",
  "arguments": {
    "listen": "0.0.0.0:8899"
  }
}

// Step 2: Configure system proxy
{
  "name": "system_proxy_set",
  "arguments": {
    "host": "127.0.0.1",
    "port": 8899
  }
}

// Step 3: Verify
{
  "name": "system_proxy_status",
  "arguments": {}
}
```

---

### Example 2: Quick Toggle On/Off

**Turn on proxy:**
```bash
# Via MCP
{
  "name": "system_proxy_set",
  "arguments": {
    "host": "127.0.0.1",
    "port": 8899
  }
}
```

**Turn off proxy:**
```bash
# Via MCP
{
  "name": "system_proxy_clear",
  "arguments": {}
}
```

---

### Example 3: Multiple Network Interfaces

**Set proxy on Ethernet:**
```json
{
  "name": "system_proxy_set",
  "arguments": {
    "host": "192.168.1.100",
    "port": 8899,
    "interface": "Ethernet"
  }
}
```

**Set proxy on Wi-Fi:**
```json
{
  "name": "system_proxy_set",
  "arguments": {
    "host": "192.168.1.100",
    "port": 8899,
    "interface": "Wi-Fi"
  }
}
```

---

## Complete Workflow via MCP

### Automated NetProxy Setup

```javascript
// 1. Check if NetProxy is running
await mcp.call("engine_status");

// 2. Start NetProxy if needed
await mcp.call("engine_start", {
  listen: "127.0.0.1:8899"
});

// 3. Configure system proxy
await mcp.call("system_proxy_set", {
  host: "127.0.0.1",
  port: 8899
});

// 4. Verify setup
const status = await mcp.call("system_proxy_status");
console.log("Proxy configured:", status);

// 5. When done, clean up
await mcp.call("system_proxy_clear");
await mcp.call("engine_stop");
```

---

## Platform Support

**macOS:** ✅ Full support via `networksetup` command  
**Windows:** ❌ Not yet implemented  
**Linux:** ❌ Not yet implemented

---

## Network Interfaces

**Common interface names:**
- `Wi-Fi` - Wireless connection
- `Ethernet` - Wired connection
- `USB Ethernet` - USB network adapter
- `Thunderbolt Ethernet` - Thunderbolt adapter

**To list all interfaces:**
```bash
networksetup -listallnetworkservices
```

---

## Automation Scripts

### Python Example

```python
import anthropic

client = anthropic.Anthropic()

# Configure proxy via MCP
response = client.messages.create(
    model="claude-sonnet-4-5",
    max_tokens=1024,
    tools=[
        {
            "type": "mcp",
            "server_name": "netproxy"
        }
    ],
    messages=[{
        "role": "user",
        "content": "Set up system proxy to 127.0.0.1:8899"
    }]
)
```

### Shell Script Example

```bash
#!/bin/bash
# Quick proxy toggle via MCP

# Start NetProxy
np-mcp --tool engine_start --args '{"listen":"127.0.0.1:8899"}'

# Set system proxy
np-mcp --tool system_proxy_set --args '{"host":"127.0.0.1","port":8899}'

echo "Proxy enabled! Press Enter to disable..."
read

# Clear proxy
np-mcp --tool system_proxy_clear

# Stop NetProxy
np-mcp --tool engine_stop
```

---

## Troubleshooting

### Proxy not working?

**Check status:**
```json
{
  "name": "system_proxy_status",
  "arguments": {}
}
```

**Verify NetProxy is listening:**
```json
{
  "name": "engine_status",
  "arguments": {}
}
```

**Check network interface:**
```bash
networksetup -listallnetworkservices
```

### Permission Issues

The MCP tools use `networksetup` which requires:
- Running on macOS
- Admin privileges (may prompt for password)
- Network interface must exist and be active

---

## Security Notes

**These tools modify system-wide network settings!**

- ✅ Only works locally (cannot be called remotely)
- ✅ Requires explicit MCP permission
- ✅ User can see each tool call
- ⚠️ Will affect ALL apps on the Mac
- ⚠️ Remember to clear proxy when done

---

## Related MCP Tools

**Engine Control:**
- `engine_start` - Start NetProxy
- `engine_stop` - Stop NetProxy
- `engine_status` - Check status

**Flow Management:**
- `flows_list` - View captured traffic
- `flows_get` - Get flow details

**Mock Management:**
- `mocks_add` - Create mock rules
- `mocks_list` - List mocks

**Complete tool list:** Run MCP server with `--list-tools`

---

**Last Updated:** 2026-10-17  
**MCP Version:** 1.0  
**Platform:** macOS only
