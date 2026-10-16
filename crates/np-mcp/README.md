# NetProxy MCP Server

Model Context Protocol server for NetProxy, enabling AI assistants to programmatically control **all** NetProxy features and system proxy configuration through natural language.

## Features

### Complete App Control (25+ tools)

- **Engine Management**: Start, stop, restart, and monitor the proxy engine
- **System Proxy Setup**: Configure macOS system-wide proxy settings  
- **ADB Integration**: Manage Android device/emulator proxy via ADB
- **iOS Simulator**: Configure iOS Simulator proxy and certificates
- **Browser Control**: Launch Chrome with proxy pre-configured
- **Flow Management**: List, search, export, and clear captured HTTP flows
- **Mock Management**: Create, update, delete, and validate mock rules
- **Session Management**: Save and restore complete sessions (flows + mocks)
- **CA Certificate**: Export, install, and manage trust status

## Installation

Add to your Claude Desktop configuration (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "netproxy": {
      "command": "/path/to/netproxy/target/release/np-mcp",
      "args": []
    }
  }
}
```

Or build from source:

```bash
cd netproxy
cargo build --release -p np-mcp
```

## Available Tools

### Engine Control

#### `engine_start`
Start the NetProxy engine with optional port configuration.

**Parameters:**
- `port` (optional): Port to listen on (default: 8899)

**Example:**
```
Start NetProxy on port 9000
```

#### `engine_stop`
Stop the NetProxy engine.

**Example:**
```
Stop the proxy engine
```

#### `engine_status`
Get current engine status (running/stopped, flows count, etc.).

**Example:**
```
What's the engine status?
```

#### `engine_restart`
Restart the engine with new configuration.

**Parameters:**
- `port` (optional): New port to listen on

**Example:**
```
Restart engine on port 8080
```

---

### System Proxy (macOS)

#### `system_proxy_set`
Configure macOS system-wide HTTP/HTTPS proxy settings.

**Parameters:**
- `host`: Proxy host (usually 127.0.0.1)
- `port`: Proxy port
- `interface` (optional): Network interface (e.g., 'Wi-Fi', auto-detected if omitted)

**Example:**
```
Set system proxy to 127.0.0.1:8899
```

#### `system_proxy_clear`
Remove macOS system proxy settings.

**Parameters:**
- `interface` (optional): Network interface

**Example:**
```
Clear system proxy
```

#### `system_proxy_status`
Check current system proxy configuration.

**Parameters:**
- `interface` (optional): Network interface

**Example:**
```
Check system proxy status
```

---

### ADB (Android)

#### `adb_devices`
List connected Android devices and emulators.

**Example:**
```
List Android devices
```

#### `adb_proxy_set`
Configure proxy on Android device via ADB.

**Parameters:**
- `device` (optional): Device ID (uses first device if omitted)
- `host`: Proxy host
- `port`: Proxy port

**Example:**
```
Set proxy on Android emulator to 10.0.2.2:8899
```

> **Note:** Use `10.0.2.2` from Android emulator to reach host's `127.0.0.1`

#### `adb_proxy_clear`
Remove proxy from Android device.

**Parameters:**
- `device` (optional): Device ID

**Example:**
```
Clear proxy from Android
```

#### `adb_proxy_status`
Check proxy status on Android device.

**Parameters:**
- `device` (optional): Device ID

**Example:**
```
Check Android proxy status
```

#### `adb_install_cert`
Install NetProxy CA certificate on Android device.

**Parameters:**
- `device` (optional): Device ID
- `cert_path`: Path to CA certificate file

**Example:**
```
Install CA cert on Android
```

---

### iOS Simulator

#### `ios_simulators_list`
List available iOS simulators.

**Example:**
```
List iOS simulators
```

#### `ios_simulator_proxy_set`
Configure proxy in iOS Simulator (provides manual instructions).

**Parameters:**
- `udid` (optional): Simulator UDID
- `host`: Proxy host
- `port`: Proxy port

**Example:**
```
Set proxy on iOS Simulator
```

> **Note:** iOS Simulator doesn't support command-line proxy configuration. This tool provides manual setup instructions.

#### `ios_simulator_proxy_clear`
Remove proxy from iOS Simulator (provides manual instructions).

**Example:**
```
Clear iOS Simulator proxy
```

#### `ios_simulator_install_cert`
Install CA certificate in iOS Simulator.

**Parameters:**
- `udid` (optional): Simulator UDID (uses booted simulator if omitted)
- `cert_path`: Path to CA certificate file

**Example:**
```
Install CA cert in iOS Simulator
```

---

### Browser Control

#### `chrome_launch_with_proxy`
Launch Chrome with NetProxy configured.

**Parameters:**
- `host`: Proxy host (usually 127.0.0.1)
- `port`: Proxy port
- `ignore_cert_errors` (optional, default: true): Ignore certificate errors
- `user_data_dir` (optional): Custom user data directory

**Example:**
```
Launch Chrome with proxy on 8899
```

#### `chrome_close`
Close Chrome browser instances.

**Example:**
```
Close Chrome
```

---

### Flow Management

#### `list_flows`
List captured HTTP flows from the current or specified run.

**Parameters:**
- `run` (optional): Run ID (default: current)
- `limit` (optional, default: 50): Maximum number of flows
- `host` (optional): Filter by host

**Example:**
```
List the last 10 flows to api.example.com
```

#### `get_flow`
Get details of a specific captured flow.

**Parameters:**
- `run` (optional): Run ID (default: current)
- `flow_id`: Flow ID to retrieve

**Example:**
```
Show me flow #42
```

#### `flows_clear`
Clear captured flows (all, by host, or by run).

**Parameters:**
- `run` (optional): Run ID (default: current)
- `host` (optional): Clear only flows matching this host

**Example:**
```
Clear all flows to api.github.com
```

#### `flows_export`
Export flows to HAR, JSON, or CSV format.

**Parameters:**
- `run` (optional): Run ID (default: current)
- `format`: Export format (har, json, csv)
- `output_path` (optional): Output file path
- `host` (optional): Filter by host

**Example:**
```
Export all flows to HAR format
```

#### `flows_search`
Search flows with advanced filters.

**Parameters:**
- `run` (optional): Run ID (default: current)
- `method` (optional): HTTP method filter
- `host` (optional): Host filter
- `path` (optional): Path filter (regex)
- `status` (optional): Response status code
- `limit` (optional, default: 50): Maximum results

**Example:**
```
Find all POST requests with 400 status
```

---

### Mock Management

#### `list_mocks`
List all active mock rules.

**Parameters:**
- `source` (optional): Filter by source type (all, static, dynamic)

**Example:**
```
List all dynamic mocks
```

#### `create_mock`
Create a new mock rule.

**Parameters:**
- `rule`: Mock rule definition (see schema)

**Example:**
```
Create a mock for /api/users that returns 200 OK
```

#### `update_mock`
Update an existing mock rule.

**Parameters:**
- `rule`: Updated mock rule (must include id)

**Example:**
```
Update mock "test-rule" to return 404
```

#### `delete_mock`
Delete a mock rule.

**Parameters:**
- `rule_id`: ID of rule to delete

**Example:**
```
Delete mock "test-rule"
```

#### `flow_to_mock`
Convert a captured flow into a mock rule.

**Parameters:**
- `run` (optional): Run ID (default: current)
- `flow_id`: Flow ID to convert
- `priority` (optional, default: 100): Priority for the generated rule
- `enabled` (optional, default: true): Enable the mock
- `match_query` (optional, default: false): Include query params in match
- `match_headers` (optional): Headers to include in match
- `add_to_registry` (optional, default: false): Immediately add to active mocks
- `write_fixtures` (optional, default: false): Write response fixtures to files

**Example:**
```
Convert flow #123 to a mock and add it to the registry
```

#### `mocks_import`
Import mock rules from file (JSON/YAML).

**Parameters:**
- `file_path`: Path to mock rules file

**Example:**
```
Import mocks from ./test-mocks.yaml
```

#### `mocks_export`
Export all mock rules to file.

**Parameters:**
- `file_path`: Output file path
- `source` (optional): Which mocks to export (all, static, dynamic)

**Example:**
```
Export all mocks to ./backup-mocks.json
```

#### `mocks_enable_all`
Enable all mock rules.

**Example:**
```
Enable all mocks
```

#### `mocks_disable_all`
Disable all mock rules.

**Example:**
```
Disable all mocks
```

#### `mocks_clear_all`
Remove all dynamic mock rules.

**Example:**
```
Clear all dynamic mocks
```

#### `validate_mocks`
Validate all mock rules and report issues.

**Example:**
```
Validate all mocks
```

---

### CA Certificate Management

#### `ca_export`
Export CA certificate to file.

**Parameters:**
- `output_path`: Where to save the certificate

**Example:**
```
Export CA certificate to ~/Desktop/netproxy-ca.crt
```

#### `ca_install_system`
Install CA certificate in macOS Keychain (requires sudo).

**Parameters:**
- `cert_path` (optional): Path to certificate (uses current CA if omitted)

**Example:**
```
Install CA certificate in system keychain
```

#### `ca_trust_status`
Check if CA certificate is trusted by the system.

**Example:**
```
Check CA trust status
```

---

### Session Management

#### `session_save`
Save current session (flows + mocks) for later.

**Parameters:**
- `name`: Session name
- `include_flows` (optional, default: true): Include captured flows
- `include_mocks` (optional, default: true): Include mock rules

**Example:**
```
Save current session as "api-test-session"
```

#### `session_load`
Load a previously saved session.

**Parameters:**
- `name`: Session name to load
- `restore_mocks` (optional, default: true): Restore mock rules

**Example:**
```
Load session "api-test-session"
```

#### `session_list`
List all saved sessions.

**Example:**
```
List all saved sessions
```

#### `session_delete`
Delete a saved session.

**Parameters:**
- `name`: Session name to delete

**Example:**
```
Delete session "old-test"
```

---

### Fixture Management

#### `create_fixture`
Create a fixture file for mock responses.

**Parameters:**
- `filename`: Fixture filename (relative to fixtures directory)
- `content`: File content
- `content_type` (optional): MIME type

**Example:**
```
Create a fixture file with user data
```

---

## Complete Workflow Examples

### Setup Android Emulator Proxy
```
1. List Android devices
2. Start NetProxy on port 8899
3. Set proxy on Android to 10.0.2.2:8899
4. Export CA certificate to /tmp/ca.crt
5. Install CA cert on Android
```

### System-Wide Proxy Setup with Chrome
```
1. Start NetProxy on port 8899
2. Set system proxy to 127.0.0.1:8899
3. Launch Chrome with proxy
```

### Capture and Mock Workflow
```
1. Start NetProxy
2. Set system proxy
3. Browse to trigger requests
4. List flows to api.example.com
5. Convert flow #42 to mock
6. Save session as "example-api-test"
7. Clear system proxy
```

### Session Replay
```
1. Load session "example-api-test"
2. Start NetProxy
3. Mocks are now active
4. Test against mocked responses
```

---

## Platform Support

- **macOS**: Full support (system proxy, iOS Simulator, Chrome)
- **Linux**: Partial support (ADB, Chrome; no system proxy)
- **Windows**: Partial support (ADB, Chrome; no system proxy)

## Requirements

- **macOS system proxy**: macOS with `networksetup` command
- **ADB tools**: Android SDK platform-tools (`adb` in PATH)
- **iOS Simulator**: Xcode or Command Line Tools (`xcrun simctl`)
- **Chrome**: Google Chrome installed in standard location

## Architecture

```
┌─────────────────────────────────────────┐
│        Claude Desktop / AI              │
└───────────────┬─────────────────────────┘
                │ MCP Protocol (stdio)
┌───────────────▼─────────────────────────┐
│         NetProxy MCP Server             │
│  ┌─────────────────────────────────┐   │
│  │  Tool Handlers                  │   │
│  │  • Engine Control               │   │
│  │  • System Proxy (macOS)         │   │
│  │  • ADB (Android)                │   │
│  │  • iOS Simulator                │   │
│  │  • Browser Control              │   │
│  │  • Session Management           │   │
│  └─────────────────────────────────┘   │
└─────┬───────────────────────────────────┘
      │ Unix Socket (JSON-RPC)
┌─────▼───────────────────────────────────┐
│        NetProxy Daemon (netproxyd)      │
│  • Proxy Engine                         │
│  • Flow Capture                         │
│  • Mock Registry                        │
│  • Control API                          │
└─────────────────────────────────────────┘
```

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for development guidelines.

## License

Licensed under either of [Apache License 2.0](../../LICENSE-APACHE) or [MIT License](../../LICENSE-MIT) at your option.
