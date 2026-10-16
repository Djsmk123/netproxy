//! Control client for connecting to netproxyd via Unix domain socket.
//!
//! This client implements the JSON-RPC 2.0 protocol over line-delimited JSON
//! and handles the authentication handshake required by the control API.

use serde_json::{json, Value};
use std::path::Path;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
};

/// Client for connecting to netproxyd's control API.
pub struct ControlClient {
    stream: UnixStream,
    next_id: u64,
}

impl ControlClient {
    /// Connect to the control socket and perform authentication handshake.
    ///
    /// This reads the token from the token file and sends the `hello` message.
    pub async fn connect(socket_path: &Path) -> anyhow::Result<Self> {
        // Read endpoint file if it exists (socket path may be redirected)
        let socket_dir = socket_path.parent().ok_or_else(|| {
            anyhow::anyhow!("socket path has no parent directory")
        })?;
        let endpoint_file = socket_dir.join("control.endpoint");
        let actual_socket = if endpoint_file.exists() {
            let endpoint_data = tokio::fs::read_to_string(&endpoint_file).await?;
            std::path::PathBuf::from(endpoint_data.trim())
        } else {
            socket_path.to_path_buf()
        };

        // Connect to socket
        let stream = UnixStream::connect(&actual_socket).await?;

        // Read token from token file
        let token_file = socket_dir.join("control.token");
        let token = tokio::fs::read_to_string(&token_file).await?.trim().to_string();

        let mut client = Self { stream, next_id: 0 };

        // Send hello message
        let hello_response = client
            .call_with_id(
                1,
                "hello",
                json!({
                    "token": token,
                    "api_version": np_model::API_VERSION,
                    "client": {
                        "name": "np-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }),
            )
            .await?;

        // Validate hello response
        if hello_response.get("api_version") != Some(&json!(np_model::API_VERSION)) {
            anyhow::bail!("API version mismatch");
        }

        client.next_id = 2; // Start from 2 after hello

        Ok(client)
    }

    /// Call a JSON-RPC method and return the result.
    pub async fn call(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        self.call_with_id(id, method, params).await
    }

    /// Internal method to call with a specific ID.
    async fn call_with_id(
        &mut self,
        id: u64,
        method: &str,
        params: Value,
    ) -> anyhow::Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        // Write request (line-delimited JSON)
        let request_str = serde_json::to_string(&request)?;
        self.stream.write_all(request_str.as_bytes()).await?;
        self.stream.write_all(b"\n").await?;
        self.stream.flush().await?;

        // Read response
        let mut reader = BufReader::new(&mut self.stream);
        let mut response_str = String::new();
        reader.read_line(&mut response_str).await?;

        let response: Value = serde_json::from_str(&response_str)?;

        // Check for errors
        if let Some(error) = response.get("error") {
            let msg = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown error");
            anyhow::bail!("RPC error: {}", msg);
        }

        // Extract result
        response
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("missing result in response"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires running daemon
    async fn test_control_client_connect() {
        let socket = std::path::PathBuf::from(
            std::env::var("NETPROXY_CONTROL_SOCKET")
                .unwrap_or_else(|_| "/tmp/netproxy-test/control.sock".to_string()),
        );

        if !socket.exists() {
            println!("Skipping test - daemon not running");
            return;
        }

        let mut client = ControlClient::connect(&socket).await.unwrap();
        let result = client.call("ping", json!({})).await.unwrap();
        assert_eq!(result.get("pong"), Some(&json!(true)));
    }
}
