//! NetProxy MCP Server
//!
//! Model Context Protocol server for NetProxy, enabling AI assistants to
//! programmatically manage HTTP mocks through natural language.

pub mod adb;
pub mod browsers;
pub mod ios_sim;
pub mod server;
pub mod sessions;
pub mod system_proxy;
pub mod tools;

pub use server::NetProxyMcpServer;
