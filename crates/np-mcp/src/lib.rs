//! NetProxy MCP Server
//!
//! Model Context Protocol server for NetProxy, enabling AI assistants to
//! programmatically manage HTTP mocks through natural language.

pub mod server;
pub mod tools;

pub use server::NetProxyMcpServer;
