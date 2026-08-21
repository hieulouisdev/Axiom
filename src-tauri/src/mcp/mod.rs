//! v1.7.0 — Model Context Protocol (MCP) server module.
//!
//! Inspired by worldmonitor's MCP integration, this module lets external
//! agents (Claude Code, Cursor, Codex, etc.) talk to Aegis AI's memory,
//! skills, world intelligence, and computer-use capabilities through a
//! simple JSON-RPC interface.
//!
//! ## Design
//!
//! - **Transport**: stdio (compatible with Claude Desktop, Cursor, etc.)
//! - **Wire**: JSON-RPC 2.0 over newline-delimited frames
//! - **Tools**: each `mcp::Tool` is a `name`, `description`, JSON schema,
//!   and a Rust handler that returns a JSON value.
//!
//! This is a minimal MCP implementation — no `mcp-sdk` dependency. The
//! spec is small enough that we can hand-roll it.

pub mod server;
pub mod tools;

pub use server::{McpServer, McpServerConfig};
pub use tools::{Tool, ToolSchema, ToolParam, ToolRegistry, registry_default};
