//! MCP tool definitions for NetProxy mock management.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// All available MCP tools for NetProxy.
pub fn get_tools() -> Vec<Tool> {
    vec![
        // ==================== Engine Control ====================
        Tool {
            name: "engine_start".to_string(),
            description: "Start the NetProxy engine with optional port configuration".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "port": {
                        "type": "integer",
                        "description": "Port to listen on (default: 8899)"
                    }
                }
            }),
        },
        Tool {
            name: "engine_stop".to_string(),
            description: "Stop the NetProxy engine".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "engine_status".to_string(),
            description: "Get current engine status (running/stopped, flows count, etc.)"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "engine_restart".to_string(),
            description: "Restart the engine with new configuration".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "port": {
                        "type": "integer",
                        "description": "Port to listen on"
                    }
                }
            }),
        },
        // ==================== System Proxy (macOS) ====================
        Tool {
            name: "system_proxy_set".to_string(),
            description: "Configure macOS system-wide proxy settings".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["host", "port"],
                "properties": {
                    "host": {
                        "type": "string",
                        "description": "Proxy host (usually 127.0.0.1)"
                    },
                    "port": {
                        "type": "integer",
                        "description": "Proxy port"
                    },
                    "interface": {
                        "type": "string",
                        "description": "Network interface (e.g., 'Wi-Fi', auto-detected if omitted)"
                    }
                }
            }),
        },
        Tool {
            name: "system_proxy_clear".to_string(),
            description: "Remove macOS system proxy settings".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "interface": {
                        "type": "string",
                        "description": "Network interface (auto-detected if omitted)"
                    }
                }
            }),
        },
        Tool {
            name: "system_proxy_status".to_string(),
            description: "Check current macOS system proxy configuration".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "interface": {
                        "type": "string",
                        "description": "Network interface (auto-detected if omitted)"
                    }
                }
            }),
        },
        // ==================== ADB (Android) ====================
        Tool {
            name: "adb_devices".to_string(),
            description: "List connected Android devices and emulators".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "adb_proxy_set".to_string(),
            description: "Configure proxy on Android device via ADB".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["host", "port"],
                "properties": {
                    "device": {
                        "type": "string",
                        "description": "Device ID (uses first device if omitted)"
                    },
                    "host": {
                        "type": "string",
                        "description": "Proxy host"
                    },
                    "port": {
                        "type": "integer",
                        "description": "Proxy port"
                    }
                }
            }),
        },
        Tool {
            name: "adb_proxy_clear".to_string(),
            description: "Remove proxy from Android device".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "device": {
                        "type": "string",
                        "description": "Device ID (uses first device if omitted)"
                    }
                }
            }),
        },
        Tool {
            name: "adb_proxy_status".to_string(),
            description: "Check proxy status on Android device".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "device": {
                        "type": "string",
                        "description": "Device ID (uses first device if omitted)"
                    }
                }
            }),
        },
        Tool {
            name: "adb_install_cert".to_string(),
            description: "Install NetProxy CA certificate on Android device".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["cert_path"],
                "properties": {
                    "device": {
                        "type": "string",
                        "description": "Device ID (uses first device if omitted)"
                    },
                    "cert_path": {
                        "type": "string",
                        "description": "Path to CA certificate file"
                    }
                }
            }),
        },
        // ==================== iOS Simulator ====================
        Tool {
            name: "ios_simulators_list".to_string(),
            description: "List available iOS simulators".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "ios_simulator_proxy_set".to_string(),
            description: "Configure proxy in iOS Simulator (provides manual instructions)"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["host", "port"],
                "properties": {
                    "udid": {
                        "type": "string",
                        "description": "Simulator UDID"
                    },
                    "host": {
                        "type": "string",
                        "description": "Proxy host"
                    },
                    "port": {
                        "type": "integer",
                        "description": "Proxy port"
                    }
                }
            }),
        },
        Tool {
            name: "ios_simulator_proxy_clear".to_string(),
            description: "Remove proxy from iOS Simulator (provides manual instructions)"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "udid": {
                        "type": "string",
                        "description": "Simulator UDID"
                    }
                }
            }),
        },
        Tool {
            name: "ios_simulator_install_cert".to_string(),
            description: "Install CA certificate in iOS Simulator".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["cert_path"],
                "properties": {
                    "udid": {
                        "type": "string",
                        "description": "Simulator UDID (uses booted simulator if omitted)"
                    },
                    "cert_path": {
                        "type": "string",
                        "description": "Path to CA certificate file"
                    }
                }
            }),
        },
        // ==================== Browser Control ====================
        Tool {
            name: "chrome_launch_with_proxy".to_string(),
            description: "Launch Chrome with NetProxy configured".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["host", "port"],
                "properties": {
                    "host": {
                        "type": "string",
                        "description": "Proxy host (usually 127.0.0.1)"
                    },
                    "port": {
                        "type": "integer",
                        "description": "Proxy port"
                    },
                    "ignore_cert_errors": {
                        "type": "boolean",
                        "default": true,
                        "description": "Ignore certificate errors"
                    },
                    "user_data_dir": {
                        "type": "string",
                        "description": "Custom user data directory"
                    }
                }
            }),
        },
        Tool {
            name: "chrome_close".to_string(),
            description: "Close Chrome browser instances".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        // ==================== Enhanced Flow Management ====================
        Tool {
            name: "flows_clear".to_string(),
            description: "Clear captured flows (all, by host, or by run)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "run": {
                        "type": "string",
                        "description": "Run ID (default: current)"
                    },
                    "host": {
                        "type": "string",
                        "description": "Clear only flows matching this host"
                    }
                }
            }),
        },
        Tool {
            name: "flows_export".to_string(),
            description: "Export flows to HAR, JSON, or CSV format".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["format"],
                "properties": {
                    "run": {
                        "type": "string",
                        "description": "Run ID (default: current)"
                    },
                    "format": {
                        "type": "string",
                        "enum": ["har", "json", "csv"],
                        "description": "Export format"
                    },
                    "output_path": {
                        "type": "string",
                        "description": "Output file path"
                    },
                    "host": {
                        "type": "string",
                        "description": "Filter by host"
                    }
                }
            }),
        },
        Tool {
            name: "flows_search".to_string(),
            description: "Search flows with advanced filters".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "run": {
                        "type": "string",
                        "description": "Run ID (default: current)"
                    },
                    "method": {
                        "type": "string",
                        "description": "HTTP method filter"
                    },
                    "host": {
                        "type": "string",
                        "description": "Host filter"
                    },
                    "path": {
                        "type": "string",
                        "description": "Path filter (regex)"
                    },
                    "status": {
                        "type": "integer",
                        "description": "Response status code"
                    },
                    "limit": {
                        "type": "integer",
                        "default": 50,
                        "description": "Maximum results"
                    }
                }
            }),
        },
        // ==================== Enhanced Mock Management ====================
        Tool {
            name: "mocks_import".to_string(),
            description: "Import mock rules from file (JSON/YAML)".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["file_path"],
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to mock rules file"
                    }
                }
            }),
        },
        Tool {
            name: "mocks_export".to_string(),
            description: "Export all mock rules to file".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["file_path"],
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Output file path"
                    },
                    "source": {
                        "type": "string",
                        "enum": ["all", "static", "dynamic"],
                        "default": "all",
                        "description": "Which mocks to export"
                    }
                }
            }),
        },
        Tool {
            name: "mocks_enable_all".to_string(),
            description: "Enable all mock rules".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "mocks_disable_all".to_string(),
            description: "Disable all mock rules".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "mocks_clear_all".to_string(),
            description: "Remove all dynamic mock rules".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        // ==================== CA Certificate Management ====================
        Tool {
            name: "ca_export".to_string(),
            description: "Export CA certificate to file".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["output_path"],
                "properties": {
                    "output_path": {
                        "type": "string",
                        "description": "Where to save the certificate"
                    }
                }
            }),
        },
        Tool {
            name: "ca_install_system".to_string(),
            description: "Install CA certificate in macOS Keychain (requires sudo)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cert_path": {
                        "type": "string",
                        "description": "Path to certificate (uses current CA if omitted)"
                    }
                }
            }),
        },
        Tool {
            name: "ca_trust_status".to_string(),
            description: "Check if CA certificate is trusted by the system".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        // ==================== Session Management ====================
        Tool {
            name: "session_save".to_string(),
            description: "Save current session (flows + mocks) for later".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Session name"
                    },
                    "include_flows": {
                        "type": "boolean",
                        "default": true,
                        "description": "Include captured flows"
                    },
                    "include_mocks": {
                        "type": "boolean",
                        "default": true,
                        "description": "Include mock rules"
                    }
                }
            }),
        },
        Tool {
            name: "session_load".to_string(),
            description: "Load a previously saved session".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Session name to load"
                    },
                    "restore_mocks": {
                        "type": "boolean",
                        "default": true,
                        "description": "Restore mock rules"
                    }
                }
            }),
        },
        Tool {
            name: "session_list".to_string(),
            description: "List all saved sessions".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "session_delete".to_string(),
            description: "Delete a saved session".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Session name to delete"
                    }
                }
            }),
        },
        // ==================== Original Tools ====================
        Tool {
            name: "list_flows".to_string(),
            description: "List captured HTTP flows from the current or specified run".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "run": {
                        "type": "string",
                        "description": "Run ID (default: current)",
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
        Tool {
            name: "get_flow".to_string(),
            description: "Get details of a specific captured flow".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["flow_id"],
                "properties": {
                    "run": {
                        "type": "string",
                        "description": "Run ID (default: current)"
                    },
                    "flow_id": {
                        "type": "integer",
                        "description": "Flow ID to retrieve"
                    }
                }
            }),
        },
        Tool {
            name: "list_mocks".to_string(),
            description: "List all active mock rules".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "enum": ["all", "static", "dynamic"],
                        "default": "all",
                        "description": "Filter by source type"
                    }
                }
            }),
        },
        Tool {
            name: "create_mock".to_string(),
            description: "Create a new mock rule".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["rule"],
                "properties": {
                    "rule": {
                        "type": "object",
                        "required": ["id", "match", "action"],
                        "description": "Mock rule definition",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Unique rule identifier"
                            },
                            "priority": {
                                "type": "integer",
                                "default": 100,
                                "description": "Rule priority (higher = checked first)"
                            },
                            "enabled": {
                                "type": "boolean",
                                "default": true
                            },
                            "description": {
                                "type": "string",
                                "description": "Human-readable description"
                            },
                            "match": {
                                "type": "object",
                                "description": "Request matching criteria",
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
                                "description": "Response action",
                                "properties": {
                                    "type": {
                                        "type": "string",
                                        "enum": ["respond", "pass", "fail"]
                                    },
                                    "status": {"type": "integer"},
                                    "headers": {"type": "object"},
                                    "body": {
                                        "description": "Response body (inline or file)",
                                        "oneOf": [
                                            {
                                                "type": "object",
                                                "properties": {"inline": {"type": "string"}}
                                            },
                                            {
                                                "type": "object",
                                                "properties": {"file": {"type": "string"}}
                                            }
                                        ]
                                    }
                                }
                            }
                        }
                    }
                }
            }),
        },
        Tool {
            name: "update_mock".to_string(),
            description: "Update an existing mock rule".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["rule"],
                "properties": {
                    "rule": {
                        "type": "object",
                        "description": "Updated mock rule (must include id)"
                    }
                }
            }),
        },
        Tool {
            name: "delete_mock".to_string(),
            description: "Delete a mock rule".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["rule_id"],
                "properties": {
                    "rule_id": {
                        "type": "string",
                        "description": "ID of rule to delete"
                    }
                }
            }),
        },
        Tool {
            name: "flow_to_mock".to_string(),
            description: "Convert a captured flow into a mock rule".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["flow_id"],
                "properties": {
                    "run": {
                        "type": "string",
                        "description": "Run ID (default: current)"
                    },
                    "flow_id": {
                        "type": "integer",
                        "description": "Flow ID to convert"
                    },
                    "priority": {
                        "type": "integer",
                        "default": 100,
                        "description": "Priority for the generated rule"
                    },
                    "enabled": {
                        "type": "boolean",
                        "default": true
                    },
                    "match_query": {
                        "type": "boolean",
                        "default": false,
                        "description": "Include query params in match criteria"
                    },
                    "match_headers": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Headers to include in match criteria"
                    },
                    "add_to_registry": {
                        "type": "boolean",
                        "default": false,
                        "description": "Immediately add to active mocks"
                    },
                    "write_fixtures": {
                        "type": "boolean",
                        "default": false,
                        "description": "Write response fixtures to files"
                    }
                }
            }),
        },
        Tool {
            name: "validate_mocks".to_string(),
            description: "Validate all mock rules and report issues".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
        Tool {
            name: "create_fixture".to_string(),
            description: "Create a fixture file for mock responses".to_string(),
            input_schema: json!({
                "type": "object",
                "required": ["filename", "content"],
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "Fixture filename (relative to fixtures directory)"
                    },
                    "content": {
                        "type": "string",
                        "description": "File content"
                    },
                    "content_type": {
                        "type": "string",
                        "description": "MIME type (optional)"
                    }
                }
            }),
        },
    ]
}
