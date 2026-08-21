//! MCP module — slim CLI version, same JSON-RPC over stdio protocol.

pub mod server;
pub mod tools;

pub use server::{McpServer, McpServerConfig};
pub use tools::{Tool, ToolSchema, ToolParam, ToolRegistry, registry_default};
