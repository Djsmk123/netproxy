//! MCP server implementation for NetProxy.
//!
//! This server implements the Model Context Protocol (MCP) over stdio,
//! exposing NetProxy's mock management capabilities as tools.

use crate::tools::get_tools;
use netproxyd::control_client::ControlClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

const E_PARSE: i32 = -32700;
const E_METHOD_NOT_FOUND: i32 = -32601;
const E_INVALID_PARAMS: i32 = -32602;
const E_INTERNAL: i32 = -32603;

pub struct NetProxyMcpServer {
    control_socket: PathBuf,
    fixtures_dir: PathBuf,
}

impl NetProxyMcpServer {
    pub fn new(control_socket: PathBuf) -> Self {
        // Default fixtures directory
        let fixtures_dir = control_socket
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("fixtures"))
            .unwrap_or_else(|| PathBuf::from("./fixtures"));

        Self {
            control_socket,
            fixtures_dir,
        }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break; // EOF
            }

            let response = match serde_json::from_str::<McpRequest>(&line) {
                Ok(req) => self.handle_request(req).await,
                Err(e) => McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(McpError {
                        code: E_PARSE,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                },
            };

            let response_str = serde_json::to_string(&response)?;
            stdout.write_all(response_str.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }

        Ok(())
    }

    async fn handle_request(&self, req: McpRequest) -> McpResponse {
        let result = match req.method.as_str() {
            "initialize" => self.handle_initialize(req.params).await,
            "tools/list" => self.handle_list_tools().await,
            "tools/call" => self.handle_call_tool(req.params).await,
            _ => Err(McpError {
                code: E_METHOD_NOT_FOUND,
                message: format!("Method not found: {}", req.method),
                data: None,
            }),
        };

        match result {
            Ok(value) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: Some(value),
                error: None,
            },
            Err(error) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: req.id,
                result: None,
                error: Some(error),
            },
        }
    }

    async fn handle_initialize(&self, _params: Value) -> Result<Value, McpError> {
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {
                "name": "netproxy-mcp",
                "version": env!("CARGO_PKG_VERSION")
            },
            "capabilities": {
                "tools": {}
            }
        }))
    }

    async fn handle_list_tools(&self) -> Result<Value, McpError> {
        let tools = get_tools();
        Ok(json!({
            "tools": tools
        }))
    }

    async fn handle_call_tool(&self, params: Value) -> Result<Value, McpError> {
        #[derive(Deserialize)]
        struct CallToolParams {
            name: String,
            #[serde(default)]
            arguments: Value,
        }

        let call_params: CallToolParams = serde_json::from_value(params).map_err(|e| McpError {
            code: E_INVALID_PARAMS,
            message: format!("Invalid params: {}", e),
            data: None,
        })?;

        let result = self
            .call_tool(&call_params.name, call_params.arguments)
            .await?;

        Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                }
            ]
        }))
    }

    async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, McpError> {
        match name {
            "create_fixture" => self.handle_create_fixture(arguments).await,
            _ => {
                // All other tools go through the control client
                let mut client = ControlClient::connect(&self.control_socket)
                    .await
                    .map_err(|e| McpError {
                        code: E_INTERNAL,
                        message: format!("Failed to connect to netproxyd: {}", e),
                        data: None,
                    })?;

                self.call_control_method(&mut client, name, arguments).await
            }
        }
    }

    async fn call_control_method(
        &self,
        client: &mut ControlClient,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, McpError> {
        let (method, params) = match tool_name {
            "list_flows" => {
                #[derive(Deserialize)]
                struct Args {
                    #[serde(default)]
                    run: Option<String>,
                    #[serde(default)]
                    limit: Option<usize>,
                    #[serde(default)]
                    host: Option<String>,
                }
                let args: Args = serde_json::from_value(arguments).map_err(|e| McpError {
                    code: E_INVALID_PARAMS,
                    message: format!("Invalid arguments: {}", e),
                    data: None,
                })?;

                let mut filter = json!({});
                if let Some(run) = args.run {
                    filter["run"] = json!(run);
                }
                if let Some(host) = args.host {
                    filter["host"] = json!(host);
                }

                (
                    "flows.list",
                    json!({
                        "filter": filter,
                        "limit": args.limit.unwrap_or(50)
                    }),
                )
            }

            "get_flow" => {
                #[derive(Deserialize)]
                struct Args {
                    #[serde(default)]
                    run: Option<String>,
                    flow_id: u64,
                }
                let args: Args = serde_json::from_value(arguments).map_err(|e| McpError {
                    code: E_INVALID_PARAMS,
                    message: format!("Invalid arguments: {}", e),
                    data: None,
                })?;

                (
                    "flows.get",
                    json!({
                        "run": args.run,
                        "flow": args.flow_id
                    }),
                )
            }

            "list_mocks" => {
                #[derive(Deserialize)]
                struct Args {
                    #[serde(default = "default_source")]
                    source: String,
                }
                fn default_source() -> String {
                    "all".to_string()
                }

                let args: Args = serde_json::from_value(arguments).unwrap_or_else(|_| Args {
                    source: "all".to_string(),
                });

                let method = match args.source.as_str() {
                    "static" => "mocks.list_static",
                    "dynamic" => "mocks.list_dynamic",
                    _ => "mocks.list",
                };

                (method, json!({}))
            }

            "create_mock" => ("mocks.add", arguments),

            "update_mock" => ("mocks.update", arguments),

            "delete_mock" => {
                #[derive(Deserialize)]
                struct Args {
                    rule_id: String,
                }
                let args: Args = serde_json::from_value(arguments).map_err(|e| McpError {
                    code: E_INVALID_PARAMS,
                    message: format!("Invalid arguments: {}", e),
                    data: None,
                })?;

                ("mocks.remove", json!({"id": args.rule_id}))
            }

            "flow_to_mock" => {
                #[derive(Deserialize)]
                struct Args {
                    #[serde(default)]
                    run: Option<String>,
                    flow_id: u64,
                    #[serde(default)]
                    priority: Option<i32>,
                    #[serde(default)]
                    enabled: Option<bool>,
                    #[serde(default)]
                    match_query: Option<bool>,
                    #[serde(default)]
                    match_headers: Option<Vec<String>>,
                    #[serde(default)]
                    add_to_registry: bool,
                    #[serde(default)]
                    write_fixtures: bool,
                }
                let args: Args = serde_json::from_value(arguments).map_err(|e| McpError {
                    code: E_INVALID_PARAMS,
                    message: format!("Invalid arguments: {}", e),
                    data: None,
                })?;

                let method = if args.add_to_registry {
                    "mocks.from_flow_and_add"
                } else {
                    "mocks.from_flow"
                };

                let mut params = json!({
                    "run": args.run,
                    "flow": args.flow_id,
                    "write_fixtures": args.write_fixtures
                });

                if let Some(priority) = args.priority {
                    params["priority"] = json!(priority);
                }
                if let Some(enabled) = args.enabled {
                    params["enabled"] = json!(enabled);
                }
                if let Some(match_query) = args.match_query {
                    params["match_query"] = json!(match_query);
                }
                if let Some(match_headers) = args.match_headers {
                    params["match_headers"] = json!(match_headers);
                }

                (method, params)
            }

            "validate_mocks" => ("mocks.validate", json!({})),

            _ => {
                return Err(McpError {
                    code: E_METHOD_NOT_FOUND,
                    message: format!("Unknown tool: {}", tool_name),
                    data: None,
                })
            }
        };

        client.call(method, params).await.map_err(|e| McpError {
            code: E_INTERNAL,
            message: format!("Control API error: {}", e),
            data: None,
        })
    }

    async fn handle_create_fixture(&self, arguments: Value) -> Result<Value, McpError> {
        #[derive(Deserialize)]
        struct Args {
            filename: String,
            content: String,
            #[serde(default)]
            content_type: Option<String>,
        }

        let args: Args = serde_json::from_value(arguments).map_err(|e| McpError {
            code: E_INVALID_PARAMS,
            message: format!("Invalid arguments: {}", e),
            data: None,
        })?;

        // Create fixtures directory if it doesn't exist
        tokio::fs::create_dir_all(&self.fixtures_dir)
            .await
            .map_err(|e| McpError {
                code: E_INTERNAL,
                message: format!("Failed to create fixtures directory: {}", e),
                data: None,
            })?;

        let path = self.fixtures_dir.join(&args.filename);

        // Prevent path traversal
        if !path.starts_with(&self.fixtures_dir) {
            return Err(McpError {
                code: E_INVALID_PARAMS,
                message: "Invalid filename (path traversal detected)".to_string(),
                data: None,
            });
        }

        tokio::fs::write(&path, args.content.as_bytes())
            .await
            .map_err(|e| McpError {
                code: E_INTERNAL,
                message: format!("Failed to write fixture: {}", e),
                data: None,
            })?;

        Ok(json!({
            "path": path.to_string_lossy(),
            "size": args.content.len(),
            "content_type": args.content_type
        }))
    }
}
