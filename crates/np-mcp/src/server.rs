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
        // Handle local tools (no daemon connection needed)
        match name {
            "create_fixture" => return self.handle_create_fixture(arguments).await,
            "system_proxy_set" => return self.handle_system_proxy_set(arguments).await,
            "system_proxy_clear" => return self.handle_system_proxy_clear(arguments).await,
            "system_proxy_status" => return self.handle_system_proxy_status(arguments).await,
            "adb_devices" => return self.handle_adb_devices().await,
            "adb_proxy_set" => return self.handle_adb_proxy_set(arguments).await,
            "adb_proxy_clear" => return self.handle_adb_proxy_clear(arguments).await,
            "adb_proxy_status" => return self.handle_adb_proxy_status(arguments).await,
            "adb_install_cert" => return self.handle_adb_install_cert(arguments).await,
            "ios_simulators_list" => return self.handle_ios_simulators_list().await,
            "ios_simulator_proxy_set" => {
                return self.handle_ios_simulator_proxy_set(arguments).await
            }
            "ios_simulator_proxy_clear" => {
                return self.handle_ios_simulator_proxy_clear(arguments).await
            }
            "ios_simulator_install_cert" => {
                return self.handle_ios_simulator_install_cert(arguments).await
            }
            "chrome_launch_with_proxy" => return self.handle_chrome_launch(arguments).await,
            "chrome_close" => return self.handle_chrome_close().await,
            "session_save" => return self.handle_session_save(arguments).await,
            "session_load" => return self.handle_session_load(arguments).await,
            "session_list" => return self.handle_session_list().await,
            "session_delete" => return self.handle_session_delete(arguments).await,
            _ => {}
        }

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

    async fn call_control_method(
        &self,
        client: &mut ControlClient,
        tool_name: &str,
        arguments: Value,
    ) -> Result<Value, McpError> {
        let (method, params) = match tool_name {
            // Engine control
            "engine_start" => {
                #[derive(Deserialize)]
                struct Args {
                    #[serde(default)]
                    port: Option<u16>,
                }
                let args: Args = serde_json::from_value(arguments).unwrap_or(Args { port: None });
                let listen = args.port.map(|p| format!("127.0.0.1:{}", p));
                ("engine.start", json!({ "listen": listen }))
            }
            "engine_stop" => ("engine.stop", json!({})),
            "engine_status" => ("engine.status", json!({})),
            "engine_restart" => {
                // Stop then start
                client.call("engine.stop", json!({})).await.ok();
                #[derive(Deserialize)]
                struct Args {
                    #[serde(default)]
                    port: Option<u16>,
                }
                let args: Args = serde_json::from_value(arguments).unwrap_or(Args { port: None });
                let listen = args.port.map(|p| format!("127.0.0.1:{}", p));
                ("engine.start", json!({ "listen": listen }))
            }

            // CA certificate
            "ca_export" => ("ca.get", json!({})),

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

    // ==================== System Proxy Handlers ====================
    async fn handle_system_proxy_set(&self, arguments: Value) -> Result<Value, McpError> {
        #[derive(Deserialize)]
        struct Args {
            host: String,
            port: u16,
            #[serde(default)]
            interface: Option<String>,
        }

        let args: Args = serde_json::from_value(arguments).map_err(|e| McpError {
            code: E_INVALID_PARAMS,
            message: format!("Invalid arguments: {}", e),
            data: None,
        })?;

        crate::system_proxy::set_system_proxy(&args.host, args.port, args.interface.as_deref())
            .map_err(|e| McpError {
                code: E_INTERNAL,
                message: e.to_string(),
                data: None,
            })
    }

    async fn handle_system_proxy_clear(&self, arguments: Value) -> Result<Value, McpError> {
        #[derive(Deserialize)]
        struct Args {
            #[serde(default)]
            interface: Option<String>,
        }

        let args: Args = serde_json::from_value(arguments).unwrap_or(Args { interface: None });

        crate::system_proxy::clear_system_proxy(args.interface.as_deref()).map_err(|e| McpError {
            code: E_INTERNAL,
            message: e.to_string(),
            data: None,
        })
    }

    async fn handle_system_proxy_status(&self, arguments: Value) -> Result<Value, McpError> {
        #[derive(Deserialize)]
        struct Args {
            #[serde(default)]
            interface: Option<String>,
        }

        let args: Args = serde_json::from_value(arguments).unwrap_or(Args { interface: None });

        crate::system_proxy::get_system_proxy_status(args.interface.as_deref()).map_err(|e| {
            McpError {
                code: E_INTERNAL,
                message: e.to_string(),
                data: None,
            }
        })
    }

    // ==================== ADB Handlers ====================
    async fn handle_adb_devices(&self) -> Result<Value, McpError> {
        crate::adb::list_devices().map_err(|e| McpError {
            code: E_INTERNAL,
            message: e.to_string(),
            data: None,
        })
    }

    async fn handle_adb_proxy_set(&self, arguments: Value) -> Result<Value, McpError> {
        #[derive(Deserialize)]
        struct Args {
            #[serde(default)]
            device: Option<String>,
            host: String,
            port: u16,
        }

        let args: Args = serde_json::from_value(arguments).map_err(|e| McpError {
            code: E_INVALID_PARAMS,
            message: format!("Invalid arguments: {}", e),
            data: None,
        })?;

        crate::adb::set_proxy(args.device.as_deref(), &args.host, args.port).map_err(|e| McpError {
            code: E_INTERNAL,
            message: e.to_string(),
            data: None,
        })
    }

    async fn handle_adb_proxy_clear(&self, arguments: Value) -> Result<Value, McpError> {
        #[derive(Deserialize)]
        struct Args {
            #[serde(default)]
            device: Option<String>,
        }

        let args: Args = serde_json::from_value(arguments).unwrap_or(Args { device: None });

        crate::adb::clear_proxy(args.device.as_deref()).map_err(|e| McpError {
            code: E_INTERNAL,
            message: e.to_string(),
            data: None,
        })
    }

    async fn handle_adb_proxy_status(&self, arguments: Value) -> Result<Value, McpError> {
        #[derive(Deserialize)]
        struct Args {
            #[serde(default)]
            device: Option<String>,
        }

        let args: Args = serde_json::from_value(arguments).unwrap_or(Args { device: None });

        crate::adb::get_proxy_status(args.device.as_deref()).map_err(|e| McpError {
            code: E_INTERNAL,
            message: e.to_string(),
            data: None,
        })
    }

    async fn handle_adb_install_cert(&self, arguments: Value) -> Result<Value, McpError> {
        #[derive(Deserialize)]
        struct Args {
            #[serde(default)]
            device: Option<String>,
            cert_path: String,
        }

        let args: Args = serde_json::from_value(arguments).map_err(|e| McpError {
            code: E_INVALID_PARAMS,
            message: format!("Invalid arguments: {}", e),
            data: None,
        })?;

        crate::adb::install_ca_cert(args.device.as_deref(), &args.cert_path).map_err(|e| McpError {
            code: E_INTERNAL,
            message: e.to_string(),
            data: None,
        })
    }

    // ==================== iOS Simulator Handlers ====================
    async fn handle_ios_simulators_list(&self) -> Result<Value, McpError> {
        crate::ios_sim::list_simulators().map_err(|e| McpError {
            code: E_INTERNAL,
            message: e.to_string(),
            data: None,
        })
    }

    async fn handle_ios_simulator_proxy_set(&self, arguments: Value) -> Result<Value, McpError> {
        #[derive(Deserialize)]
        struct Args {
            #[serde(default)]
            udid: Option<String>,
            host: String,
            port: u16,
        }

        let args: Args = serde_json::from_value(arguments).map_err(|e| McpError {
            code: E_INVALID_PARAMS,
            message: format!("Invalid arguments: {}", e),
            data: None,
        })?;

        crate::ios_sim::set_proxy(args.udid.as_deref(), &args.host, args.port).map_err(|e| {
            McpError {
                code: E_INTERNAL,
                message: e.to_string(),
                data: None,
            }
        })
    }

    async fn handle_ios_simulator_proxy_clear(&self, arguments: Value) -> Result<Value, McpError> {
        #[derive(Deserialize)]
        struct Args {
            #[serde(default)]
            udid: Option<String>,
        }

        let args: Args = serde_json::from_value(arguments).unwrap_or(Args { udid: None });

        crate::ios_sim::clear_proxy(args.udid.as_deref()).map_err(|e| McpError {
            code: E_INTERNAL,
            message: e.to_string(),
            data: None,
        })
    }

    async fn handle_ios_simulator_install_cert(&self, arguments: Value) -> Result<Value, McpError> {
        #[derive(Deserialize)]
        struct Args {
            #[serde(default)]
            udid: Option<String>,
            cert_path: String,
        }

        let args: Args = serde_json::from_value(arguments).map_err(|e| McpError {
            code: E_INVALID_PARAMS,
            message: format!("Invalid arguments: {}", e),
            data: None,
        })?;

        crate::ios_sim::install_ca_cert(args.udid.as_deref(), &args.cert_path).map_err(|e| {
            McpError {
                code: E_INTERNAL,
                message: e.to_string(),
                data: None,
            }
        })
    }

    // ==================== Browser Handlers ====================
    async fn handle_chrome_launch(&self, arguments: Value) -> Result<Value, McpError> {
        #[derive(Deserialize)]
        struct Args {
            host: String,
            port: u16,
            #[serde(default)]
            ignore_cert_errors: Option<bool>,
            #[serde(default)]
            user_data_dir: Option<String>,
        }

        let args: Args = serde_json::from_value(arguments).map_err(|e| McpError {
            code: E_INVALID_PARAMS,
            message: format!("Invalid arguments: {}", e),
            data: None,
        })?;

        crate::browsers::launch_with_proxy(
            "chrome",
            &args.host,
            args.port,
            args.ignore_cert_errors.unwrap_or(true),
            args.user_data_dir.as_deref(),
        )
        .map_err(|e| McpError {
            code: E_INTERNAL,
            message: e.to_string(),
            data: None,
        })
    }

    async fn handle_chrome_close(&self) -> Result<Value, McpError> {
        crate::browsers::close_browser("chrome").map_err(|e| McpError {
            code: E_INTERNAL,
            message: e.to_string(),
            data: None,
        })
    }

    // ==================== Session Handlers ====================
    async fn handle_session_save(&self, arguments: Value) -> Result<Value, McpError> {
        #[derive(Deserialize)]
        struct Args {
            name: String,
            #[serde(default)]
            include_flows: Option<bool>,
            #[serde(default)]
            include_mocks: Option<bool>,
        }

        let args: Args = serde_json::from_value(arguments).map_err(|e| McpError {
            code: E_INVALID_PARAMS,
            message: format!("Invalid arguments: {}", e),
            data: None,
        })?;

        // Get data dir from control socket path
        let data_dir = self
            .control_socket
            .parent()
            .and_then(|p| p.parent())
            .ok_or_else(|| McpError {
                code: E_INTERNAL,
                message: "could not determine data directory".to_string(),
                data: None,
            })?;

        // Collect flows and mocks
        let mut client = ControlClient::connect(&self.control_socket)
            .await
            .map_err(|e| McpError {
                code: E_INTERNAL,
                message: format!("Failed to connect to netproxyd: {}", e),
                data: None,
            })?;

        let flows = if args.include_flows.unwrap_or(true) {
            let result = client
                .call("flows.list", json!({ "limit": 1000 }))
                .await
                .map_err(|e| McpError {
                    code: E_INTERNAL,
                    message: e.to_string(),
                    data: None,
                })?;
            result["flows"].as_array().cloned().unwrap_or_default()
        } else {
            vec![]
        };

        let mocks = if args.include_mocks.unwrap_or(true) {
            let result = client
                .call("mocks.list", json!({}))
                .await
                .map_err(|e| McpError {
                    code: E_INTERNAL,
                    message: e.to_string(),
                    data: None,
                })?;
            result["rules"].as_array().cloned().unwrap_or_default()
        } else {
            vec![]
        };

        crate::sessions::save_session(data_dir, &args.name, mocks, flows, None).map_err(|e| {
            McpError {
                code: E_INTERNAL,
                message: e.to_string(),
                data: None,
            }
        })
    }

    async fn handle_session_load(&self, arguments: Value) -> Result<Value, McpError> {
        #[derive(Deserialize)]
        struct Args {
            name: String,
            #[serde(default)]
            restore_mocks: Option<bool>,
        }

        let args: Args = serde_json::from_value(arguments).map_err(|e| McpError {
            code: E_INVALID_PARAMS,
            message: format!("Invalid arguments: {}", e),
            data: None,
        })?;

        let data_dir = self
            .control_socket
            .parent()
            .and_then(|p| p.parent())
            .ok_or_else(|| McpError {
                code: E_INTERNAL,
                message: "could not determine data directory".to_string(),
                data: None,
            })?;

        let session =
            crate::sessions::load_session(data_dir, &args.name).map_err(|e| McpError {
                code: E_INTERNAL,
                message: e.to_string(),
                data: None,
            })?;

        // Restore mocks if requested
        if args.restore_mocks.unwrap_or(true) {
            let mut client = ControlClient::connect(&self.control_socket)
                .await
                .map_err(|e| McpError {
                    code: E_INTERNAL,
                    message: format!("Failed to connect to netproxyd: {}", e),
                    data: None,
                })?;

            for mock in &session.mocks {
                client.call("mocks.add", json!({ "rule": mock })).await.ok();
            }
        }

        Ok(json!({
            "success": true,
            "session": session.name,
            "mocks_restored": session.mocks.len(),
            "flows_available": session.flows.len()
        }))
    }

    async fn handle_session_list(&self) -> Result<Value, McpError> {
        let data_dir = self
            .control_socket
            .parent()
            .and_then(|p| p.parent())
            .ok_or_else(|| McpError {
                code: E_INTERNAL,
                message: "could not determine data directory".to_string(),
                data: None,
            })?;

        crate::sessions::list_sessions(data_dir).map_err(|e| McpError {
            code: E_INTERNAL,
            message: e.to_string(),
            data: None,
        })
    }

    async fn handle_session_delete(&self, arguments: Value) -> Result<Value, McpError> {
        #[derive(Deserialize)]
        struct Args {
            name: String,
        }

        let args: Args = serde_json::from_value(arguments).map_err(|e| McpError {
            code: E_INVALID_PARAMS,
            message: format!("Invalid arguments: {}", e),
            data: None,
        })?;

        let data_dir = self
            .control_socket
            .parent()
            .and_then(|p| p.parent())
            .ok_or_else(|| McpError {
                code: E_INTERNAL,
                message: "could not determine data directory".to_string(),
                data: None,
            })?;

        crate::sessions::delete_session(data_dir, &args.name).map_err(|e| McpError {
            code: E_INTERNAL,
            message: e.to_string(),
            data: None,
        })
    }
}
