//! v1.7.0 — Minimal MCP server (JSON-RPC 2.0 over stdio).
//!
//! Implements the subset of the Model Context Protocol that external
//! agents need to discover and call Aegis AI's tools:
//!
//! - `initialize`         — handshake, returns server info + capabilities
//! - `tools/list`         — lists registered tools
//! - `tools/call`         — invokes a tool
//! - `ping`               — health check
//! - `shutdown`           — graceful shutdown
//!
//! The server reads newline-delimited JSON from stdin and writes
//! newline-delimited JSON responses to stdout. This is the wire format
//! every MCP client (Claude Desktop, Cursor, Codex, etc.) supports.

use std::io::{BufRead, Write};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::error::Result;

use super::tools::ToolRegistry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub version: String,
    pub protocol_version: String,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            name: "aegis-ai".into(),
            version: "1.7.0".into(),
            protocol_version: "2025-06-18".into(),
        }
    }
}

pub struct McpServer {
    config: McpServerConfig,
    registry: Arc<Mutex<ToolRegistry>>,
}

impl McpServer {
    pub fn new(config: McpServerConfig, registry: ToolRegistry) -> Self {
        Self {
            config,
            registry: Arc::new(Mutex::new(registry)),
        }
    }

    /// Run the server: read newline-delimited JSON-RPC from stdin,
    /// dispatch each request, and write responses to stdout.
    ///
    /// This is a **blocking** entry point — call it from a dedicated
    /// thread or `tokio::task::spawn_blocking`.
    pub fn run_blocking(&self) -> Result<()> {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        for line in stdin.lock().lines() {
            let line = line.map_err(|e| crate::error::Error::Other(format!("stdin: {e}")))?;
            if line.trim().is_empty() {
                continue;
            }
            let req: Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(e) => {
                    let resp = error_response(Value::Null, -32700, &format!("parse error: {e}"));
                    writeln!(out, "{}", resp)?;
                    continue;
                }
            };
            let resp = self.dispatch_sync(req);
            if let Some(r) = resp {
                writeln!(out, "{}", r)?;
                out.flush()?;
            }
        }
        Ok(())
    }

    /// Dispatch a single JSON-RPC request. Returns `None` for notifications
    /// (requests without an `id` field).
    fn dispatch_sync(&self, req: Value) -> Option<String> {
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let params = req.get("params").cloned().unwrap_or(Value::Null);
        let rt = tokio::runtime::Runtime::new().ok()?;
        let result = rt.block_on(self.handle_method(method, params));
        match result {
            Ok(v) => {
                if id == Value::Null {
                    return None;
                }
                Some(serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": v,
                })).unwrap_or_else(|_| "{}".into()))
            }
            Err(e) => {
                if id == Value::Null {
                    return None;
                }
                Some(error_response(id, -32000, &format!("{e}")))
            }
        }
    }

    async fn handle_method(&self, method: &str, params: Value) -> Result<Value> {
        match method {
            "initialize" => Ok(json!({
                "protocolVersion": self.config.protocol_version,
                "serverInfo": {
                    "name": self.config.name,
                    "version": self.config.version,
                },
                "capabilities": {
                    "tools": {}
                }
            })),
            "ping" => Ok(json!({})),
            "tools/list" => {
                let reg = self.registry.lock().await;
                let tools: Vec<Value> = reg
                    .list()
                    .iter()
                    .map(|t| {
                        json!({
                            "name": t.name,
                            "description": t.description,
                            "inputSchema": {
                                "type": "object",
                                "properties": t.schema.params.iter().filter_map(|p| {
                                    Some((p.name.clone(), json!({
                                        "type": p.ty,
                                        "description": p.description,
                                    })))
                                }).collect::<serde_json::Map<String, Value>>(),
                                "required": t.schema.params.iter().filter(|p| p.required).map(|p| p.name.clone()).collect::<Vec<_>>(),
                            }
                        })
                    })
                    .collect();
                Ok(json!({ "tools": tools }))
            }
            "tools/call" => {
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| crate::error::Error::Other("tools/call requires 'name'".into()))?;
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let reg = self.registry.lock().await;
                let result = reg.call(name, args).await?;
                Ok(json!({
                    "content": [{
                        "type": "text",
                        "text": serde_json::to_string_pretty(&result).unwrap_or_default()
                    }]
                }))
            }
            "shutdown" => Ok(json!({})),
            _ => Err(crate::error::Error::Other(format!("unknown method: {method}"))),
        }
    }
}

fn error_response(id: Value, code: i64, message: &str) -> String {
    serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    }))
    .unwrap_or_else(|_| "{}".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn initialize_returns_server_info() {
        let s = McpServer::new(McpServerConfig::default(), super::super::tools::registry_default());
        let r = s.handle_method("initialize", json!({})).await.unwrap();
        assert_eq!(r["serverInfo"]["name"], "aegis-ai");
        assert_eq!(r["protocolVersion"], "2025-06-18");
    }

    #[tokio::test]
    async fn tools_list_returns_schema() {
        let s = McpServer::new(McpServerConfig::default(), super::super::tools::registry_default());
        let r = s.handle_method("tools/list", json!({})).await.unwrap();
        let tools = r["tools"].as_array().unwrap();
        assert!(tools.len() >= 9);
        assert!(tools.iter().any(|t| t["name"] == "memory_search"));
    }

    #[tokio::test]
    async fn tools_call_invokes_handler() {
        let s = McpServer::new(McpServerConfig::default(), super::super::tools::registry_default());
        let r = s.handle_method("tools/call", json!({
            "name": "memory_search",
            "arguments": { "query": "rust" }
        })).await.unwrap();
        let text = r["content"][0]["text"].as_str().unwrap();
        let parsed: Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["echo"]["query"], "rust");
    }
}
