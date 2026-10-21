# NetProxy MCP Tools Reference

Complete reference for all MCP (Model Context Protocol) tools available in NetProxy for AI automation.

## Table of Contents

- [Engine Control](#engine-control)
- [Session Management](#session-management)
- [Request Operations](#request-operations)
- [Breakpoints](#breakpoints)
- [Mock Management](#mock-management)
- [Flow Management](#flow-management)
- [Device Configuration](#device-configuration)
- [CA Certificate](#ca-certificate)

---

## Engine Control

### `engine_start`
Start the NetProxy engine with optional port configuration.

**Parameters:**
- `port` (integer, optional): Port to listen on (default: 8899)

**Example:**
```json
{
  "name": "engine_start",
  "arguments": {
    "port": 8899
  }
}
```

### `engine_stop`
Stop the NetProxy engine.

**Parameters:** None

### `engine_status`
Get current engine status (running/stopped, flows count, etc.)

**Parameters:** None

**Returns:**
```json
{
  "running": true,
  "listen": "127.0.0.1:8899",
  "flows_count": 42,
  "uptime_seconds": 3600
}
```

### `engine_restart`
Restart the engine with new configuration.

**Parameters:**
- `port` (integer, optional): Port to listen on

---

## Session Management

### `session_list`
List all saved sessions.

**Parameters:** None

**Returns:**
```json
{
  "sessions": [
    {
      "name": "api-testing",
      "created_at": "2026-10-21T10:30:00Z",
      "flows_count": 125,
      "has_mocks": true
    }
  ]
}
```

### `session_save`
Save current session (flows + mocks) for later.

**Parameters:**
- `name` (string, required): Session name
- `include_flows` (boolean, default: true): Include captured flows
- `include_mocks` (boolean, default: true): Include mock rules

**Example:**
```json
{
  "name": "engine_start",
  "arguments": {
    "name": "my-api-session",
    "include_flows": true,
    "include_mocks": true
  }
}
```

### `session_load`
Load a previously saved session.

**Parameters:**
- `name` (string, required): Session name to load
- `restore_mocks` (boolean, default: true): Restore mock rules

### `session_delete`
Delete a saved session.

**Parameters:**
- `name` (string, required): Session name to delete

### `session_import_har`
Import HAR file as a new session.

**Parameters:**
- `har_path` (string, required): Path to HAR file to import

**Example:**
```json
{
  "name": "session_import_har",
  "arguments": {
    "har_path": "/Users/me/Downloads/network-log.har"
  }
}
```

**Returns:**
```json
{
  "run_id": "abc123def456",
  "flows_imported": 42,
  "duration_ms": 150
}
```

### `sessions_delete_bulk`
Delete multiple sessions at once.

**Parameters:**
- `run_ids` (array of strings, required): Array of run IDs to delete

**Example:**
```json
{
  "name": "sessions_delete_bulk",
  "arguments": {
    "run_ids": ["run-abc123", "run-def456", "run-ghi789"]
  }
}
```

---

## Request Operations

### `request_save_file`
Save request and/or response to file from a captured flow.

**Parameters:**
- `run` (string, optional): Run ID (default: current)
- `flow_id` (integer, required): Flow ID to save
- `output_path` (string, required): Output file path
- `part` (string, default: "both"): Which part to save ("request", "response", or "both")

**Example:**
```json
{
  "name": "request_save_file",
  "arguments": {
    "flow_id": 123,
    "output_path": "/tmp/request.json",
    "part": "both"
  }
}
```

### `request_copy_curl`
Generate a cURL command from a captured request.

**Parameters:**
- `run` (string, optional): Run ID (default: current)
- `flow_id` (integer, required): Flow ID to convert

**Example:**
```json
{
  "name": "request_copy_curl",
  "arguments": {
    "flow_id": 123
  }
}
```

**Returns:**
```json
{
  "curl": "curl -X POST 'https://api.example.com/users' -H 'Content-Type: application/json' -d '{\"name\":\"Alice\"}'"
}
```

---

## Breakpoints

Request breakpoints allow you to pause and inspect requests matching specific URL patterns.

### `breakpoint_add`
Add a request breakpoint on URL pattern (wildcards supported).

**Parameters:**
- `pattern` (string, required): URL pattern to match (supports `*` wildcards)
- `enabled` (boolean, default: true): Enable breakpoint immediately

**Example:**
```json
{
  "name": "breakpoint_add",
  "arguments": {
    "pattern": "https://api.example.com/users/*",
    "enabled": true
  }
}
```

**Returns:**
```json
{
  "id": "bp-abc123",
  "pattern": "https://api.example.com/users/*",
  "enabled": true
}
```

### `breakpoint_list`
List all request breakpoints.

**Parameters:** None

**Returns:**
```json
{
  "breakpoints": [
    {
      "id": "bp-abc123",
      "pattern": "https://api.example.com/users/*",
      "enabled": true,
      "hits": 15
    }
  ]
}
```

### `breakpoint_remove`
Remove a breakpoint by ID.

**Parameters:**
- `id` (string, required): Breakpoint ID to remove

**Example:**
```json
{
  "name": "breakpoint_remove",
  "arguments": {
    "id": "bp-abc123"
  }
}
```

### `breakpoint_toggle`
Enable or disable a breakpoint.

**Parameters:**
- `id` (string, required): Breakpoint ID
- `enabled` (boolean, required): New enabled state

**Example:**
```json
{
  "name": "breakpoint_toggle",
  "arguments": {
    "id": "bp-abc123",
    "enabled": false
  }
}
```

---

## Mock Management

### Mock Map Local

#### `mock_map_local`
Create a mock that maps a URL to a local file.

**Parameters:**
- `pattern` (string, required): URL pattern to match (supports `*` wildcards)
- `file_path` (string, required): Local file to serve
- `method` (string, optional): HTTP method to match
- `enabled` (boolean, default: true): Enable rule immediately
- `priority` (integer, default: 100): Rule priority

**Example:**
```json
{
  "name": "mock_map_local",
  "arguments": {
    "pattern": "https://api.example.com/config.json",
    "file_path": "/Users/me/dev/mock-config.json",
    "method": "GET",
    "enabled": true,
    "priority": 100
  }
}
```

**Returns:**
```json
{
  "id": "map-local-abc123",
  "rule": { ... }
}
```

### Mock Map Remote

#### `mock_map_remote`
Create a mock that redirects one URL to another.

**Parameters:**
- `pattern` (string, required): URL pattern to match (supports `*` wildcards)
- `target_url` (string, required): Target URL to redirect to
- `method` (string, optional): HTTP method to match
- `enabled` (boolean, default: true): Enable rule immediately
- `priority` (integer, default: 100): Rule priority

**Example:**
```json
{
  "name": "mock_map_remote",
  "arguments": {
    "pattern": "https://api.production.com/*",
    "target_url": "https://api.staging.com",
    "method": "GET",
    "enabled": true,
    "priority": 100
  }
}
```

### Mock from Flow

#### `propose_mock`
Convert a captured flow into a mock rule.

**Parameters:**
- `run` (string, optional): Run ID (default: current)
- `flow_id` (integer, required): Flow ID to convert
- `priority` (integer, default: 100): Rule priority
- `enabled` (boolean, default: true): Enable rule immediately
- `match_query` (boolean, default: false): Include query params in match criteria
- `match_headers` (array of strings, optional): Headers to include in match criteria
- `add_to_registry` (boolean, default: false): Immediately add to active mocks
- `write_fixtures` (boolean, default: false): Write response fixtures to files

**Example:**
```json
{
  "name": "propose_mock",
  "arguments": {
    "flow_id": 123,
    "priority": 100,
    "enabled": true,
    "add_to_registry": true,
    "write_fixtures": true
  }
}
```

### Other Mock Tools

#### `mocks_import`
Import mock rules from file (JSON/YAML).

**Parameters:**
- `file_path` (string, required): Path to mock rules file

#### `mocks_export`
Export all mock rules to file.

**Parameters:**
- `file_path` (string, required): Output file path
- `source` (string, default: "all"): Which mocks to export ("all", "static", "dynamic")

#### `mocks_enable_all`
Enable all mock rules.

**Parameters:** None

#### `mocks_disable_all`
Disable all mock rules.

**Parameters:** None

#### `mocks_clear_all`
Remove all dynamic mock rules.

**Parameters:** None

#### `validate_mocks`
Validate all mock rules and report issues.

**Parameters:** None

---

## Flow Management

### `list_flows`
List captured HTTP flows from the current or specified run.

**Parameters:**
- `run` (string, optional): Run ID (default: current)
- `limit` (integer, default: 50): Maximum results
- `host` (string, optional): Filter by host

**Example:**
```json
{
  "name": "list_flows",
  "arguments": {
    "limit": 100,
    "host": "api.example.com"
  }
}
```

### `get_flow`
Get detailed information about a specific flow.

**Parameters:**
- `run` (string, optional): Run ID (default: current)
- `flow_id` (integer, required): Flow ID

### `flows_export`
Export flows to HAR, JSON, or CSV format.

**Parameters:**
- `run` (string, optional): Run ID (default: current)
- `format` (string, required): Export format ("har", "json", "csv")
- `output_path` (string, optional): Output file path
- `host` (string, optional): Filter by host

**Example:**
```json
{
  "name": "flows_export",
  "arguments": {
    "format": "har",
    "output_path": "/Users/me/Downloads/session.har"
  }
}
```

### `flows_search`
Search flows with advanced filters.

**Parameters:**
- `run` (string, optional): Run ID (default: current)
- `method` (string, optional): HTTP method filter
- `host` (string, optional): Host filter
- `path` (string, optional): Path filter (regex)
- `status` (integer, optional): Response status code
- `limit` (integer, default: 50): Maximum results

### `flows_clear`
Clear captured flows (all, by host, or by run).

**Parameters:**
- `run` (string, default: "current"): Run ID
- `host` (string, optional): Clear only flows matching this host

---

## Device Configuration

### macOS System Proxy

#### `system_proxy_set`
Configure macOS system-wide proxy settings.

**Parameters:**
- `host` (string, required): Proxy host (usually 127.0.0.1)
- `port` (integer, required): Proxy port
- `interface` (string, optional): Network interface (auto-detected if omitted)

#### `system_proxy_clear`
Remove macOS system proxy settings.

**Parameters:**
- `interface` (string, optional): Network interface (auto-detected if omitted)

#### `system_proxy_status`
Check current macOS system proxy configuration.

**Parameters:**
- `interface` (string, optional): Network interface (auto-detected if omitted)

### Android (ADB)

#### `adb_devices`
List connected Android devices and emulators.

#### `adb_proxy_set`
Configure proxy on Android device via ADB.

**Parameters:**
- `device` (string, optional): Device ID (uses first device if omitted)
- `host` (string, required): Proxy host
- `port` (integer, required): Proxy port

#### `adb_proxy_clear`
Remove proxy from Android device.

#### `adb_proxy_status`
Check proxy status on Android device.

#### `adb_install_cert`
Install NetProxy CA certificate on Android device.

### iOS Simulator

#### `ios_simulators_list`
List available iOS simulators.

#### `ios_simulator_proxy_set`
Configure proxy in iOS Simulator (provides manual instructions).

#### `ios_simulator_proxy_clear`
Remove proxy from iOS Simulator (provides manual instructions).

#### `ios_simulator_install_cert`
Install CA certificate in iOS Simulator.

### Browser Control

#### `chrome_launch_with_proxy`
Launch Chrome with NetProxy configured.

**Parameters:**
- `host` (string, required): Proxy host (usually 127.0.0.1)
- `port` (integer, required): Proxy port
- `ignore_cert_errors` (boolean, default: true): Ignore certificate errors
- `user_data_dir` (string, optional): Custom user data directory

#### `chrome_close`
Close Chrome browser instances.

---

## CA Certificate

### `ca_export`
Export CA certificate to file.

**Parameters:**
- `output_path` (string, required): Where to save the certificate

### `ca_install_system`
Install CA certificate in macOS Keychain (requires sudo).

**Parameters:**
- `cert_path` (string, optional): Path to certificate (uses current CA if omitted)

### `ca_trust_status`
Check if CA certificate is trusted by the system.

**Parameters:** None

---

## Usage Examples

### Automation Workflow Example

```javascript
// 1. Start NetProxy
await mcp.callTool("engine_start", { port: 8899 });

// 2. Configure system proxy
await mcp.callTool("system_proxy_set", {
  host: "127.0.0.1",
  port: 8899
});

// 3. Add a breakpoint to inspect user API calls
await mcp.callTool("breakpoint_add", {
  pattern: "https://api.example.com/users/*",
  enabled: true
});

// 4. Create a mock to redirect production to staging
await mcp.callTool("mock_map_remote", {
  pattern: "https://api.production.com/*",
  target_url: "https://api.staging.com",
  enabled: true
});

// 5. Run your tests...

// 6. Export session for later analysis
await mcp.callTool("flows_export", {
  format: "har",
  output_path: "/tmp/test-session.har"
});

// 7. Save session with mocks
await mcp.callTool("session_save", {
  name: "regression-test-baseline",
  include_flows: true,
  include_mocks: true
});

// 8. Clean up
await mcp.callTool("system_proxy_clear", {});
await mcp.callTool("engine_stop", {});
```

### Convert Captured Flow to Mock

```javascript
// 1. Find the flow you want to mock
const flows = await mcp.callTool("list_flows", {
  host: "api.example.com"
});

// 2. Convert flow to mock
const mock = await mcp.callTool("propose_mock", {
  flow_id: flows[0].id,
  priority: 100,
  enabled: true,
  add_to_registry: true,
  write_fixtures: true
});

// 3. The mock is now active and will respond to matching requests
```

---

## Error Handling

All tools return standard JSON-RPC 2.0 error responses:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32603,
    "message": "Failed to connect to netproxyd: Connection refused",
    "data": null
  }
}
```

**Common Error Codes:**
- `-32700`: Parse error
- `-32601`: Method not found
- `-32602`: Invalid params
- `-32603`: Internal error

---

## See Also

- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [NetProxy Control API](./CONTROL-API.md)
- [Mock Rules Reference](./MOCK-RULES.md)
