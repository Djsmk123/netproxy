# Connect NetProxy MCP to Claude Code CLI

Integrate NetProxy with Claude Code terminal/IDE for AI-powered proxy control.

---

## Quick Setup

### Step 1: Build the MCP Server

```bash
cd /Users/smkwinner/Desktop/workspace/netproxy
cargo build --release --bin np-mcp
```

---

### Step 2: Configure Claude Code

**Option A: Project-Level Configuration (Recommended)**

Add to `.claude/settings.json` in your project:

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

**Option B: User-Level Configuration**

Add to `~/.claude/settings.json`:

```bash
# Edit global config
code ~/.claude/settings.json
```

Same JSON structure as above.

---

### Step 3: Test the Connection

**In Claude Code terminal:**

```bash
# Start a session
claude

# Test MCP tools
> "List all available NetProxy MCP tools"

# You should see all tools listed!
```

---

## Auto-Setup Script

**Run this to configure automatically:**

```bash
cd /Users/smkwinner/Desktop/workspace/netproxy
./scripts/setup-claude-code-mcp.sh
```

This will:
1. Build the MCP server
2. Configure Claude Code automatically
3. Verify the setup
4. Show example commands

---

## What You Can Do in Claude Code

### Terminal Commands via Natural Language

**Start NetProxy:**
```bash
claude
> "Start NetProxy on port 8899 and configure my Mac's proxy"
```

**Analyze Traffic:**
```bash
> "Show me the last 20 HTTP requests captured by NetProxy"
> "Find all requests to api.stripe.com"
> "What was the response for flow 42?"
```

**Create Mocks:**
```bash
> "Create a mock that returns test-user.json for /api/users/me"
> "Convert flow 123 into a mock rule"
```

**System Setup:**
```bash
> "Set up my Mac to use NetProxy as system proxy"
> "Configure my Android device to use NetProxy"
> "Clear all proxy settings"
```

---

## IDE Integration (VS Code / JetBrains)

### VS Code Claude Extension

**Settings location:**
`.vscode/settings.json` or User Settings

```json
{
  "claude.mcpServers": {
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

### JetBrains IDE

**Settings → Tools → Claude Code → MCP Servers**

Add:
- Name: `netproxy`
- Command: `/Users/smkwinner/Desktop/workspace/netproxy/target/release/np-mcp`
- Data Dir: `/Users/smkwinner/Library/Application Support/NetProxy`

---

## Project-Specific Configuration

### For This NetProxy Project

**`.claude/settings.json`:**

```json
{
  "mcpServers": {
    "netproxy": {
      "command": "${workspaceFolder}/target/release/np-mcp",
      "args": [],
      "env": {
        "NETPROXY_DATA_DIR": "${workspaceFolder}/test-data"
      }
    }
  },
  "permissions": {
    "allow": [
      "mcp:netproxy:*"
    ]
  }
}
```

This uses:
- `${workspaceFolder}` - Project root directory
- Test data directory for isolated testing
- Auto-approval for all NetProxy MCP tools

---

### For Other Projects

**When working on iOS/Android apps:**

```json
{
  "mcpServers": {
    "netproxy": {
      "command": "/Users/smkwinner/Desktop/workspace/netproxy/target/release/np-mcp",
      "env": {
        "NETPROXY_DATA_DIR": "/Users/smkwinner/Library/Application Support/NetProxy"
      }
    }
  }
}
```

---

## Example Workflows

### iOS App Development

```bash
cd ~/Projects/MyiOSApp
claude

> "I'm testing my iOS app. Set up NetProxy for me"

Claude will:
1. Start NetProxy engine
2. Configure Mac system proxy
3. Give you iOS device setup instructions
4. Tell you how to view captured traffic
```

### API Testing

```bash
cd ~/Projects/api-integration
claude

> "I need to mock the /auth endpoint to return a 401 error"

Claude will:
1. Create a mock rule
2. Show you the rule configuration
3. Tell you how to verify it works
```

### Traffic Analysis

```bash
> "Analyze all requests my app made and show me which ones failed"

Claude will:
1. List all captured flows
2. Filter for failed requests
3. Show details of each failure
4. Suggest what might be wrong
```

---

## Available MCP Tools

### Engine Control
- `engine_start` - Start NetProxy daemon
- `engine_stop` - Stop NetProxy daemon
- `engine_status` - Get current status

### System Proxy (macOS)
- `system_proxy_set` - Configure system proxy ⭐
- `system_proxy_clear` - Remove proxy settings
- `system_proxy_status` - Check current config

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

### Mobile Device Setup
- `adb_set_proxy` - Configure Android device
- `adb_clear_proxy` - Remove Android proxy
- `adb_list_devices` - List connected devices
- `ios_sim_set_proxy` - Configure iOS Simulator
- `ios_sim_clear_proxy` - Clear iOS Simulator

### Browser Control
- `chrome_launch` - Launch Chrome with proxy
- `firefox_launch` - Launch Firefox with proxy

### Session Management
- `runs_list` - List recording sessions
- `runs_delete` - Delete a session
- `sessions_save` - Save session state
- `sessions_load` - Load saved session

**See all tools:**
```bash
./target/release/np-mcp --list-tools
```

---

## Configuration Options

### With Debug Logging

```json
{
  "mcpServers": {
    "netproxy": {
      "command": "/path/to/np-mcp",
      "args": ["--verbose"],
      "env": {
        "NETPROXY_DATA_DIR": "/path/to/data",
        "RUST_LOG": "debug",
        "RUST_BACKTRACE": "1"
      }
    }
  }
}
```

### Multiple Environments

```json
{
  "mcpServers": {
    "netproxy-dev": {
      "command": "/path/to/np-mcp",
      "env": {
        "NETPROXY_DATA_DIR": "/tmp/netproxy-dev"
      }
    },
    "netproxy-prod": {
      "command": "/path/to/np-mcp",
      "env": {
        "NETPROXY_DATA_DIR": "~/Library/Application Support/NetProxy"
      }
    }
  }
}
```

### With Auto-Permissions

```json
{
  "mcpServers": {
    "netproxy": {
      "command": "/path/to/np-mcp",
      "env": {
        "NETPROXY_DATA_DIR": "~/Library/Application Support/NetProxy"
      }
    }
  },
  "permissions": {
    "allow": [
      "mcp:netproxy:engine_*",
      "mcp:netproxy:flows_list",
      "mcp:netproxy:system_proxy_status"
    ]
  }
}
```

---

## Troubleshooting

### Tools not showing up

**Check config location:**
```bash
# Project-level
cat .claude/settings.json

# User-level
cat ~/.claude/settings.json
```

**Verify binary:**
```bash
ls -la /Users/smkwinner/Desktop/workspace/netproxy/target/release/np-mcp
./target/release/np-mcp --version
```

**Restart Claude Code session:**
```bash
# Exit current session
exit

# Start new session
claude
```

### Permission denied

**Make binary executable:**
```bash
chmod +x /Users/smkwinner/Desktop/workspace/netproxy/target/release/np-mcp
```

### Tools work but return errors

**Check if NetProxy daemon is running:**
```bash
ps aux | grep netproxyd
```

**Check control socket:**
```bash
ls -la ~/Library/Application\ Support/NetProxy/run/
```

**Test MCP directly:**
```bash
./target/release/np-mcp --test
```

---

## Advanced Usage

### Use in Scripts

```bash
#!/bin/bash
# Auto-setup NetProxy in CI/CD

claude --non-interactive << 'EOF'
Start NetProxy on port 8899
Configure system proxy
Run my test suite
Show me all failed requests
Clear proxy settings
Stop NetProxy
EOF
```

### Hooks Integration

**Add to `.claude/settings.json`:**

```json
{
  "mcpServers": {
    "netproxy": { ... }
  },
  "hooks": {
    "on-session-start": [
      "echo 'NetProxy MCP available. Try: Start NetProxy on port 8899'"
    ]
  }
}
```

### Custom Commands

**Create project-specific commands:**

```json
{
  "mcpServers": {
    "netproxy": { ... }
  },
  "commands": {
    "start-proxy": {
      "description": "Start NetProxy and configure system",
      "prompt": "Start NetProxy on port 8899 and set up my Mac's system proxy"
    },
    "stop-proxy": {
      "description": "Stop NetProxy and clear proxy",
      "prompt": "Stop NetProxy and clear my Mac's proxy settings"
    }
  }
}
```

Then use:
```bash
claude /start-proxy
claude /stop-proxy
```

---

## Comparison: Claude Desktop vs Claude Code

| Feature | Claude Desktop | Claude Code |
|---------|---------------|-------------|
| Config Location | `~/Library/Application Support/Claude/` | `.claude/settings.json` or `~/.claude/` |
| Scope | Global | Project or User |
| Best For | General use | Development workflows |
| Auto-reload | On restart | On session start |
| Permissions | Per-tool approval | Can pre-approve |
| Hooks | No | Yes |
| Custom Commands | No | Yes |

---

## Update MCP Server

**When NetProxy is updated:**

```bash
cd /Users/smkwinner/Desktop/workspace/netproxy

# Pull changes
git pull

# Rebuild MCP
cargo build --release --bin np-mcp

# Restart Claude Code session
# (Config stays the same)
```

---

## Uninstall

**Remove from project:**
```bash
# Edit .claude/settings.json
# Remove the "netproxy" entry from mcpServers
```

**Remove from user config:**
```bash
# Edit ~/.claude/settings.json
# Remove the "netproxy" entry
```

---

## Security Notes

**Project-level config is safer:**
- Only active in that project
- Can commit to version control
- Team members get same setup
- Isolated data directories

**User-level config:**
- Active in all Claude Code sessions
- Not shared in git
- Uses global data directory
- Good for personal workflows

---

**Last Updated:** 2026-10-17  
**Claude Code Version:** Latest  
**MCP Protocol:** v1.0
