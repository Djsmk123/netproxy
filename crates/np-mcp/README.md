# NetProxy MCP Server

Model Context Protocol server for NetProxy, enabling AI assistants to manage HTTP mocks programmatically through natural language.

## Overview

The NetProxy MCP server exposes NetProxy's mock management capabilities as MCP tools, allowing AI assistants like Claude to:

- List and inspect captured HTTP flows
- Create, update, and delete mock rules
- Convert captured flows into reusable mocks
- Validate mock configurations
- Manage fixture files

## Installation

Build from source:

```bash
cargo build --release --package np-mcp
```

The binary will be at `target/release/np-mcp`.

## Configuration

### Claude Desktop (macOS)

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "netproxy": {
      "command": "/path/to/np-mcp",
      "env": {
        "NETPROXY_CONTROL_SOCKET": "/Users/username/Library/Application Support/NetProxy/run/control.sock"
      }
    }
  }
}
```

### Environment Variables

- `NETPROXY_CONTROL_SOCKET`: Path to the NetProxy daemon's control socket
  - Default: `~/Library/Application Support/NetProxy/run/control.sock` (macOS)

## Available Tools

### Flow Inspection

**`list_flows`** - List captured HTTP flows
- Parameters:
  - `run` (optional): Run ID (default: "current")
  - `limit` (optional): Maximum number of flows (default: 50)
  - `host` (optional): Filter by hostname

**`get_flow`** - Get details of a specific flow
- Parameters:
  - `flow_id` (required): Flow ID
  - `run` (optional): Run ID (default: "current")

### Mock Management

**`list_mocks`** - List all active mock rules
- Parameters:
  - `source` (optional): Filter by source ("all", "static", "dynamic")

**`create_mock`** - Create a new mock rule
- Parameters:
  - `rule` (required): Mock rule definition with `id`, `match`, and `action`

**`update_mock`** - Update an existing mock rule
- Parameters:
  - `rule` (required): Updated mock rule (must include `id`)

**`delete_mock`** - Delete a mock rule
- Parameters:
  - `rule_id` (required): ID of rule to delete

### Flow Conversion

**`flow_to_mock`** - Convert a captured flow into a mock rule
- Parameters:
  - `flow_id` (required): Flow ID to convert
  - `run` (optional): Run ID (default: "current")
  - `priority` (optional): Rule priority (default: 100)
  - `enabled` (optional): Enable rule (default: true)
  - `match_query` (optional): Include query params in match (default: false)
  - `match_headers` (optional): Array of headers to match
  - `add_to_registry` (optional): Immediately add to active mocks (default: false)
  - `write_fixtures` (optional): Write fixtures to files (default: false)

### Validation & Fixtures

**`validate_mocks`** - Validate all mock rules and report issues

**`create_fixture`** - Create a fixture file for mock responses
- Parameters:
  - `filename` (required): Fixture filename
  - `content` (required): File content
  - `content_type` (optional): MIME type

## Usage Examples

### Example 1: AI-Assisted Mock Creation

**User:** "Create a mock for the GitHub user API that returns my profile"

**Claude:**
1. Calls `list_flows` to find recent GitHub API requests
2. Identifies `/users/djsmk123` flow
3. Calls `flow_to_mock` with `add_to_registry: true`
4. Confirms: "Created mock rule that will respond with your cached profile data"

### Example 2: Testing Edge Cases

**User:** "Mock the payment API to return a 429 rate limit error"

**Claude:**
1. Calls `create_mock` with appropriate configuration
2. Confirms: "Mock created. Payment API will now return 429."

### Example 3: Converting Multiple Flows

**User:** "Convert all the API calls I just made to test fixtures"

**Claude:**
1. Calls `list_flows` with appropriate filters
2. Iterates through flows, calling `flow_to_mock` for each
3. Reports: "Created 5 mock rules from captured flows"

## Architecture

The MCP server communicates with the NetProxy daemon via its Unix domain socket control API:

```
Claude Desktop → np-mcp (stdio) → netproxyd control socket → NetProxy daemon
```

All mock operations are authenticated and go through the same control API used by the CLI and GUI.

## Development

Run tests:

```bash
cargo test --package np-mcp
```

The integration tests require a running NetProxy daemon.

## Protocol

This server implements the [Model Context Protocol](https://modelcontextprotocol.io/) version 2024-11-05.

## License

See the root LICENSE file.
