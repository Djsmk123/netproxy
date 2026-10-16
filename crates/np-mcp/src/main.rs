//! NetProxy MCP Server Binary
//!
//! This binary runs an MCP server on stdio, exposing NetProxy's mock
//! management capabilities as tools for AI assistants.

use np_mcp::NetProxyMcpServer;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Read control socket path from environment or use default
    let control_socket = std::env::var("NETPROXY_CONTROL_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            // Default path on macOS
            let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("NetProxy")
                .join("run")
                .join("control.sock")
        });

    // Initialize and run the MCP server
    let server = NetProxyMcpServer::new(control_socket);
    server.run().await?;

    Ok(())
}
