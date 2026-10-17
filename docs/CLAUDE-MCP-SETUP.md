# Connect NetProxy MCP to Claude Desktop

Complete guide to integrate NetProxy with Claude Desktop via MCP.

---

## Quick Setup

### Step 1: Build the MCP Server

```bash
cd /Users/smkwinner/Desktop/workspace/netproxy
cargo build --release --bin np-mcp
```

The MCP server binary will be at: `target/release/np-mcp`

---

### Step 2: Configure Claude Desktop

**Location of config file:**
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`
- Linux: `~/.config/Claude/claude_desktop_config.json`

**Create or edit the config file:**

```json
{
  "mcpServers": {
    "netproxy": {
      "command": "/Users/smkwinner/Desktop/workspace/netproxy/target/release/np-mcp",
      "args": [],
      "env": {
        "NETPROXY_DATA_DIR": "/Users/smkwinner/Library/Application Support/NetProxy"
      }
    }
  }
}
```

**Important:** Replace `/Users/smkwinner` with your actual home directory path!

---

### Step 3: Restart Claude Desktop

1. Quit Claude Desktop completely
2. Reopen Claude Desktop
3. The NetProxy MCP server will auto-start

---

### Step 4: Test the Connection

**In a new Claude conversation, try:**

> "List all NetProxy MCP tools available"

Claude should respond with all available tools like:
- engine_start
- engine_stop
- flows_list
- system_proxy_set
- etc.

---

## Alternative Configuration Options

### Option 1: Using the Development Binary

```json
{
  "mcpServers": {
    "netproxy": {
      "command": "/Users/smkwinner/Desktop/workspace/netproxy/target/debug/np-mcp",
      "args": ["--verbose"],
      "env": {
        "NETPROXY_DATA_DIR": "/Users/smkwinner/Library/Application Support/NetProxy",
        "RUST_LOG": "debug"
      }
    }
  }
}
```

### Option 2: Custom Data Directory

```json
{
  "mcpServers": {
    "netproxy": {
      "command": "/Users/smkwinner/Desktop/workspace/netproxy/target/release/np-mcp",
      "args": [],
      "env": {
        "NETPROXY_DATA_DIR": "/Users/smkwinner/Desktop/workspace/netproxy/test-data"
      }
    }
  }
}
```

### Option 3: Multiple Instances

```json
{
  "mcpServers": {
    "netproxy-dev": {
      "command": "/Users/smkwinner/Desktop/workspace/netproxy/target/release/np-mcp",
      "env": {
        "NETPROXY_DATA_DIR": "/tmp/netproxy-dev"
      }
    },
    "netproxy-prod": {
      "command": "/Users/smkwinner/Desktop/workspace/netproxy/target/release/np-mcp",
      "env": {
        "NETPROXY_DATA_DIR": "/Users/smkwinner/Library/Application Support/NetProxy"
      }
    }
  }
}
```

---

## What Claude Can Do Now

### Proxy Control

**Start NetProxy:**
> "Start NetProxy on port 8899"

**Stop NetProxy:**
> "Stop NetProxy"

**Check status:**
> "What's the status of NetProxy?"

### System Proxy Setup

**Configure macOS proxy:**
> "Set up my Mac to use NetProxy as the system proxy"

**Clear proxy:**
> "Remove the proxy settings from my Mac"

### Traffic Analysis

**List captured requests:**
> "Show me the last 10 HTTP requests NetProxy captured"

**Find specific traffic:**
> "Show me all requests to api.example.com"

**Get request details:**
> "Show me the details of flow 42"

### Mock Management

**Create mocks:**
> "Create a mock that serves test-data.json for requests to /api/users"

**List mocks:**
> "Show me all active mock rules"

**From captured traffic:**
> "Convert flow 123 into a mock rule"

### Session Management

**List sessions:**
> "Show me all NetProxy recording sessions"

**Delete old data:**
> "Delete session abc123"

**Export data:**
> "Export the current session as HAR"

---

## Example Conversations

### Example 1: Quick Setup

**You:** "I need to test my iOS app. Set up NetProxy for me."

**Claude:** "I'll set up NetProxy for you. Let me:
1. Start the NetProxy engine
2. Configure your Mac's system proxy

[Uses tools: engine_start, system_proxy_set]

Done! NetProxy is now running on port 8899 and your Mac is configured to use it. You can now configure your iOS device to use your Mac's IP address with port 8899."

---

### Example 2: Traffic Analysis

**You:** "Show me all API calls my app made in the last session"

**Claude:** "Let me check your recent traffic...

[Uses tools: flows_list, flows_get]

I found 47 requests to api.yourapp.com:
- 23 GET requests to /users
- 15 POST requests to /auth
- 9 GET requests to /profile

Would you like me to show details for any specific request?"

---

### Example 3: Mock Creation

**You:** "I need to test the app when the API returns an error. Can you help?"

**Claude:** "I can create a mock rule for you. Which endpoint should return an error?

[After you respond]

I'll create a mock that returns a 500 error for /api/data:

[Uses tool: mocks_add]

Done! Now requests to /api/data will return a 500 error with a test error message. Test your error handling!"

---

## Available MCP Tools

### Engine Control
- `engine_start` - Start NetProxy daemon
- `engine_stop` - Stop NetProxy daemon  
- `engine_status` - Get current status

### Flow Management
- `flows_list` - List captured HTTP flows
- `flows_get` - Get detailed flow information
- `flows_filter` - Filter flows by criteria

### Mock Rules
- `mocks_add` - Create new mock rule
- `mocks_list` - List all mocks
- `mocks_update` - Update existing mock
- `mocks_remove` - Delete mock
- `mocks_from_flow` - Convert flow to mock

### System Proxy (macOS)
- `system_proxy_set` - Configure system proxy
- `system_proxy_clear` - Remove proxy settings
- `system_proxy_status` - Check proxy config

### Session Management
- `runs_list` - List recording sessions
- `runs_delete` - Delete a session

### iOS/Android Setup
- `adb_set_proxy` - Configure Android proxy
- `adb_clear_proxy` - Remove Android proxy
- `ios_sim_set_proxy` - Configure iOS Simulator
- `ios_sim_clear_proxy` - Clear iOS Simulator proxy

### Browser Control
- `chrome_launch` - Launch Chrome with proxy
- `firefox_launch` - Launch Firefox with proxy

**Full tool list:**
```bash
# Get complete list with descriptions
./target/release/np-mcp --list-tools
```

---

## Troubleshooting

### Claude doesn't see NetProxy tools

**Check config file location:**
```bash
# macOS
cat ~/Library/Application\ Support/Claude/claude_desktop_config.json

# Should show netproxy configuration
```

**Check binary exists:**
```bash
ls -la /Users/smkwinner/Desktop/workspace/netproxy/target/release/np-mcp
```

**Restart Claude Desktop:**
- Completely quit (Cmd+Q)
- Wait 2 seconds
- Reopen

### Tools are listed but don't work

**Check if NetProxy daemon is accessible:**
```bash
# Check if control socket exists
ls -la /Users/smkwinner/Library/Application\ Support/NetProxy/run/

# Try connecting directly
./target/release/np-mcp --test-connection
```

**Check logs:**
```bash
# MCP server logs
tail -f ~/Library/Logs/Claude/mcp-netproxy.log

# NetProxy daemon logs
tail -f /Users/smkwinner/Library/Application\ Support/NetProxy/netproxy-daemon.log
```

### Permission denied

**Make binary executable:**
```bash
chmod +x /Users/smkwinner/Desktop/workspace/netproxy/target/release/np-mcp
```

**Check data directory permissions:**
```bash
ls -la ~/Library/Application\ Support/NetProxy/
```

---

## Advanced Configuration

### With Logging

```json
{
  "mcpServers": {
    "netproxy": {
      "command": "/Users/smkwinner/Desktop/workspace/netproxy/target/release/np-mcp",
      "args": ["--log-level", "debug"],
      "env": {
        "NETPROXY_DATA_DIR": "/Users/smkwinner/Library/Application Support/NetProxy",
        "RUST_LOG": "np_mcp=debug",
        "RUST_BACKTRACE": "1"
      }
    }
  }
}
```

### With Custom Port

```json
{
  "mcpServers": {
    "netproxy": {
      "command": "/Users/smkwinner/Desktop/workspace/netproxy/target/release/np-mcp",
      "args": [],
      "env": {
        "NETPROXY_DATA_DIR": "/Users/smkwinner/Library/Application Support/NetProxy",
        "NETPROXY_PORT": "9999"
      }
    }
  }
}
```

---

## Security Notes

**The MCP server can:**
- ✅ Control NetProxy daemon
- ✅ Read captured traffic
- ✅ Create/modify mock rules
- ✅ Configure system proxy settings
- ⚠️ Access all HTTP/HTTPS traffic

**Best practices:**
- Only use on trusted networks
- Don't share captured traffic
- Clear sensitive data after testing
- Disable when not needed

**Claude Desktop will:**
- Ask permission before using tools
- Show you each tool call
- Let you approve/deny each action

---

## Update MCP Server

**When you update NetProxy:**

```bash
cd /Users/smkwinner/Desktop/workspace/netproxy

# Pull latest changes
git pull

# Rebuild MCP server
cargo build --release --bin np-mcp

# Restart Claude Desktop
# (Config stays the same, just picks up new binary)
```

---

## Uninstall

**Remove from Claude Desktop:**

1. Edit config file:
```bash
code ~/Library/Application\ Support/Claude/claude_desktop_config.json
```

2. Remove the `netproxy` entry

3. Restart Claude Desktop

**Or completely remove Claude's MCP config:**
```bash
rm ~/Library/Application\ Support/Claude/claude_desktop_config.json
```

---

**Last Updated:** 2026-10-17  
**Claude Desktop Version:** Latest  
**MCP Protocol:** v1.0
