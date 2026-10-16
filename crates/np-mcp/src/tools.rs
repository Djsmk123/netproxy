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
