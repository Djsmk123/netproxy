# M4 Phase 4: MCP Integration for Programmatic Mock Authoring

**Status:** 📋 Designed  
**Date:** 2026-10-17

## Overview

Expose NetProxy's mock management capabilities through the Model Context Protocol (MCP), enabling AI assistants to programmatically create, manage, and inspect HTTP mocks during development workflows.

## Goals

1. MCP server exposing NetProxy mock operations as tools
2. Read captured flows and convert them to mocks
3. Create/update/delete mock rules programmatically
4. Validate mock configurations
5. Query active mocks and their hit counts
6. Enable AI-assisted API testing workflows

## Non-Goals (Defer to Later)

- Full proxy control (start/stop engine) via MCP
- Real-time flow streaming via MCP
- Mock rule templates/libraries
- Multi-user/team collaboration features

---

## MCP Server Architecture

### Server Configuration

**File:** `crates/np-mcp/Cargo.toml` (new crate)

```toml
[package]
name = "np-mcp"
version = "0.1.0"
edition = "2021"

[dependencies]
np-model = { path = "../np-model" }
np-mock = { path = "../np-mock" }
np-store = { path = "../np-store" }
netproxyd = { path = "../netproxyd" }
mcp-rs = "0.1"  # MCP SDK
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
anyhow = "1"
```

### Tool Definitions

```rust
// crates/np-mcp/src/tools.rs

pub const TOOLS: &[ToolDefinition] = &[
    // Flow inspection
    ToolDefinition {
        name: "list_flows",
        description: "List captured HTTP flows from the current or specified run",
        input_schema: json!({
            "type": "object",
            "properties": {
                "run": {
                    "type": "string",
                    "description": "Run ID (default: current)",
                    "default": "current"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of flows",
                    "default": 50
                },
                "host": {
                    "type": "string",
                    "description": "Filter by host"
                }
            }
        }),
    },
    
    ToolDefinition {
        name: "get_flow",
        description: "Get details of a specific captured flow",
        input_schema: json!({
            "type": "object",
            "required": ["flow_id"],
            "properties": {
                "run": {"type": "string", "default": "current"},
                "flow_id": {"type": "integer"}
            }
        }),
    },
    
    // Mock management
    ToolDefinition {
        name: "list_mocks",
        description: "List all active mock rules",
        input_schema: json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "enum": ["all", "static", "dynamic"],
                    "default": "all"
                }
            }
        }),
    },
    
    ToolDefinition {
        name: "create_mock",
        description: "Create a new mock rule",
        input_schema: json!({
            "type": "object",
            "required": ["rule"],
            "properties": {
                "rule": {
                    "type": "object",
                    "required": ["id", "match", "action"],
                    "properties": {
                        "id": {"type": "string"},
                        "priority": {"type": "integer", "default": 100},
                        "enabled": {"type": "boolean", "default": true},
                        "description": {"type": "string"},
                        "match": {
                            "type": "object",
                            "properties": {
                                "method": {"type": "string"},
                                "host": {"type": "string"},
                                "path": {"type": "string"},
                                "query": {"type": "object"},
                                "headers": {"type": "object"}
                            }
                        },
                        "action": {
                            "type": "object",
                            "required": ["type"],
                            "properties": {
                                "type": {"type": "string", "enum": ["respond", "pass", "fail"]},
                                "status": {"type": "integer"},
                                "headers": {"type": "object"},
                                "body": {
                                    "oneOf": [
                                        {"type": "object", "properties": {"inline": {"type": "string"}}},
                                        {"type": "object", "properties": {"file": {"type": "string"}}}
                                    ]
                                }
                            }
                        }
                    }
                }
            }
        }),
    },
    
    ToolDefinition {
        name: "update_mock",
        description: "Update an existing mock rule",
        input_schema: json!({
            "type": "object",
            "required": ["rule_id", "rule"],
            "properties": {
                "rule_id": {"type": "string"},
                "rule": {"type": "object"}  // Same as create_mock
            }
        }),
    },
    
    ToolDefinition {
        name: "delete_mock",
        description: "Delete a mock rule",
        input_schema: json!({
            "type": "object",
            "required": ["rule_id"],
            "properties": {
                "rule_id": {"type": "string"}
            }
        }),
    },
    
    // Flow to mock conversion
    ToolDefinition {
        name: "flow_to_mock",
        description: "Convert a captured flow into a mock rule",
        input_schema: json!({
            "type": "object",
            "required": ["flow_id"],
            "properties": {
                "run": {"type": "string", "default": "current"},
                "flow_id": {"type": "integer"},
                "priority": {"type": "integer", "default": 100},
                "enabled": {"type": "boolean", "default": true},
                "match_query": {"type": "boolean", "default": false},
                "match_headers": {
                    "type": "array",
                    "items": {"type": "string"}
                },
                "pattern_specificity": {
                    "type": "string",
                    "enum": ["exact", "moderate", "broad"],
                    "description": "Pattern matching specificity (requires Phase 3)"
                },
                "add_to_registry": {
                    "type": "boolean",
                    "default": false,
                    "description": "If true, immediately add to active mocks"
                }
            }
        }),
    },
    
    // Validation
    ToolDefinition {
        name: "validate_mocks",
        description: "Validate all mock rules and report issues",
        input_schema: json!({"type": "object"}),
    },
    
    // Fixture management
    ToolDefinition {
        name: "create_fixture",
        description: "Create a fixture file for mock responses",
        input_schema: json!({
            "type": "object",
            "required": ["filename", "content"],
            "properties": {
                "filename": {"type": "string"},
                "content": {"type": "string"},
                "content_type": {"type": "string"}
            }
        }),
    },
];
```

### MCP Server Implementation

```rust
// crates/np-mcp/src/server.rs

use mcp_rs::{Server, ServerBuilder, ToolResult};
use netproxyd::control::ControlClient;
use std::sync::Arc;

pub struct NetProxyMcpServer {
    control_socket: PathBuf,
}

impl NetProxyMcpServer {
    pub fn new(control_socket: PathBuf) -> Self {
        Self { control_socket }
    }
    
    pub async fn run(self) -> anyhow::Result<()> {
        let server = ServerBuilder::new()
            .with_name("netproxy-mcp")
            .with_version(env!("CARGO_PKG_VERSION"))
            .with_tools(TOOLS.to_vec())
            .with_tool_handler(Arc::new(self))
            .build();
        
        server.run_stdio().await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl ToolHandler for NetProxyMcpServer {
    async fn call_tool(&self, name: &str, arguments: Value) -> anyhow::Result<ToolResult> {
        let mut client = ControlClient::connect(&self.control_socket).await?;
        
        match name {
            "list_flows" => {
                let params = serde_json::from_value(arguments)?;
                let result = client.call("flows.list", params).await?;
                Ok(ToolResult::success(result))
            }
            
            "get_flow" => {
                let params = serde_json::from_value(arguments)?;
                let result = client.call("flows.get", params).await?;
                Ok(ToolResult::success(result))
            }
            
            "list_mocks" => {
                #[derive(Deserialize)]
                struct Params {
                    #[serde(default = "default_source")]
                    source: String,
                }
                fn default_source() -> String { "all".to_string() }
                
                let p: Params = serde_json::from_value(arguments)?;
                let method = match p.source.as_str() {
                    "static" => "mocks.list_static",
                    "dynamic" => "mocks.list_dynamic",
                    _ => "mocks.list",
                };
                let result = client.call(method, json!({})).await?;
                Ok(ToolResult::success(result))
            }
            
            "create_mock" => {
                let result = client.call("mocks.add", arguments).await?;
                Ok(ToolResult::success(result))
            }
            
            "update_mock" => {
                let result = client.call("mocks.update", arguments).await?;
                Ok(ToolResult::success(result))
            }
            
            "delete_mock" => {
                let result = client.call("mocks.remove", arguments).await?;
                Ok(ToolResult::success(result))
            }
            
            "flow_to_mock" => {
                #[derive(Deserialize)]
                struct Params {
                    add_to_registry: Option<bool>,
                    #[serde(flatten)]
                    other: Value,
                }
                
                let p: Params = serde_json::from_value(arguments)?;
                let method = if p.add_to_registry.unwrap_or(false) {
                    "mocks.from_flow_and_add"
                } else {
                    "mocks.from_flow"
                };
                let result = client.call(method, p.other).await?;
                Ok(ToolResult::success(result))
            }
            
            "validate_mocks" => {
                let result = client.call("mocks.validate", json!({})).await?;
                Ok(ToolResult::success(result))
            }
            
            "create_fixture" => {
                #[derive(Deserialize)]
                struct Params {
                    filename: String,
                    content: String,
                    content_type: Option<String>,
                }
                let p: Params = serde_json::from_value(arguments)?;
                
                // Write fixture to fixtures directory
                let fixtures_dir = /* get from daemon config or default */;
                let path = fixtures_dir.join(&p.filename);
                tokio::fs::write(&path, p.content.as_bytes()).await?;
                
                Ok(ToolResult::success(json!({
                    "path": path.to_string_lossy(),
                    "size": p.content.len()
                })))
            }
            
            _ => Ok(ToolResult::error(format!("Unknown tool: {}", name))),
        }
    }
}
```

### Binary Entry Point

```rust
// crates/np-mcp/src/main.rs

use np_mcp::NetProxyMcpServer;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let control_socket = std::env::var("NETPROXY_CONTROL_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::data_local_dir()
                .unwrap()
                .join("NetProxy")
                .join("run")
                .join("control.sock")
        });
    
    let server = NetProxyMcpServer::new(control_socket);
    server.run().await
}
```

## MCP Configuration

### Claude Desktop Configuration

**File:** `~/Library/Application Support/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "netproxy": {
      "command": "/path/to/netproxy-mcp",
      "env": {
        "NETPROXY_CONTROL_SOCKET": "/Users/username/Library/Application Support/NetProxy/run/control.sock"
      }
    }
  }
}
```

### VS Code MCP Configuration

```json
{
  "mcp.servers": [
    {
      "name": "netproxy",
      "command": "/path/to/netproxy-mcp",
      "env": {
        "NETPROXY_CONTROL_SOCKET": "~/.netproxy/control.sock"
      }
    }
  ]
}
```

## Usage Examples

### Example 1: AI-Assisted Mock Creation

**User:** "Create a mock for the GitHub user API that returns my profile"

**Claude:**
1. Calls `list_flows` to find recent GitHub API requests
2. Identifies `/users/djsmk123` flow
3. Calls `flow_to_mock` with `add_to_registry: true`
4. Confirms: "Created mock rule `github-user-profile` that will respond with your cached profile data"

### Example 2: Testing Edge Cases

**User:** "Mock the payment API to return a 429 rate limit error"

**Claude:**
1. Calls `create_mock` with:
   ```json
   {
     "rule": {
       "id": "payment-rate-limit",
       "match": {"host": "api.stripe.com", "path": "/v1/charges"},
       "action": {
         "type": "respond",
         "status": 429,
         "headers": {"retry-after": "60"},
         "body": {"inline": "{\"error\":\"rate_limit_exceeded\"}"}
       }
     }
   }
   ```
2. Confirms: "Mock created. Payment API will now return 429 for the next minute."

### Example 3: Validating Test Setup

**User:** "Check if there are any issues with my mocks"

**Claude:**
1. Calls `validate_mocks`
2. Reports: "Found 2 warnings: Rule `api-v1-users` has low priority (10) and may be shadowed by `api-wildcard` (priority 100)"

### Example 4: Converting Multiple Flows

**User:** "Convert all the API calls I just made to test fixtures"

**Claude:**
1. Calls `list_flows` with `host: "api.example.com"`
2. Iterates through flows, calling `flow_to_mock` for each
3. Reports: "Created 5 mock rules from captured flows with fixtures saved to ./fixtures/"

## Control Client Library

To support the MCP server, create a reusable control client:

```rust
// crates/netproxyd/src/control_client.rs

use tokio::net::UnixStream;
use serde_json::{json, Value};

pub struct ControlClient {
    stream: UnixStream,
    next_id: u64,
}

impl ControlClient {
    pub async fn connect(socket_path: &Path) -> anyhow::Result<Self> {
        let stream = UnixStream::connect(socket_path).await?;
        Ok(Self { stream, next_id: 1 })
    }
    
    pub async fn call(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        
        // Write request
        let request_str = serde_json::to_string(&request)?;
        self.stream.write_all(request_str.as_bytes()).await?;
        self.stream.write_all(b"\n").await?;
        
        // Read response
        let mut reader = BufReader::new(&mut self.stream);
        let mut response_str = String::new();
        reader.read_line(&mut response_str).await?;
        
        let response: Value = serde_json::from_str(&response_str)?;
        
        if let Some(error) = response.get("error") {
            anyhow::bail!("RPC error: {}", error);
        }
        
        Ok(response["result"].clone())
    }
}
```

## Testing

### Unit Tests

```rust
#[tokio::test]
async fn mcp_tool_list_flows() {
    let server = setup_test_server().await;
    
    let result = server.call_tool("list_flows", json!({"limit": 10})).await.unwrap();
    
    assert!(result.is_success());
    let flows = result.content[0].as_array().unwrap();
    assert!(flows.len() <= 10);
}

#[tokio::test]
async fn mcp_tool_create_mock() {
    let server = setup_test_server().await;
    
    let result = server.call_tool("create_mock", json!({
        "rule": {
            "id": "test-mock",
            "match": {"host": "test.local"},
            "action": {"type": "pass"}
        }
    })).await.unwrap();
    
    assert!(result.is_success());
}
```

### Integration Test (with Claude)

Manual test using Claude Desktop:

1. Start NetProxy daemon
2. Configure MCP server in Claude Desktop
3. Restart Claude Desktop
4. Chat: "List my recent HTTP requests"
5. Verify Claude calls `list_flows` tool
6. Chat: "Create a mock for the first one"
7. Verify Claude calls `flow_to_mock` with correct flow_id

## Documentation

### README for MCP Server

```markdown
# NetProxy MCP Server

Model Context Protocol server for NetProxy, enabling AI assistants to manage HTTP mocks programmatically.

## Installation

```bash
cargo install --path crates/np-mcp
```

## Configuration

Add to your Claude Desktop config:

```json
{
  "mcpServers": {
    "netproxy": {
      "command": "netproxy-mcp"
    }
  }
}
```

## Available Tools

- `list_flows` - List captured HTTP requests
- `get_flow` - Get flow details
- `list_mocks` - List active mocks
- `create_mock` - Create new mock rule
- `update_mock` - Update existing mock
- `delete_mock` - Remove mock
- `flow_to_mock` - Convert flow to mock
- `validate_mocks` - Check for issues
- `create_fixture` - Write fixture file

## Examples

See [MCP_EXAMPLES.md](./MCP_EXAMPLES.md) for conversation examples.
```

## Success Criteria

- ✅ MCP server exposes all mock management operations
- ✅ Claude can list flows and create mocks from conversation
- ✅ Flow-to-mock conversion works via MCP
- ✅ Fixture creation works programmatically
- ✅ Validation results are readable by AI
- ✅ Error messages are clear and actionable
- ✅ Integration test with Claude Desktop passes

## Timeline Estimate

- **MCP server scaffold:** 0.5 days
- **Tool implementations:** 1 day
- **Control client:** 0.5 days
- **Testing:** 0.5 days
- **Documentation:** 0.5 days

**Total:** ~3 days

---

**Related:**
- M4 Phase 1: Dynamic Registry (✅ Complete)
- M4 Phase 2: Flow Conversion (✅ Complete)
- M4 Phase 3: Smart Patterns (📋 Designed)
