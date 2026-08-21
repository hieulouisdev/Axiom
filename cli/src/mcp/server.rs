//! MCP server — JSON-RPC over stdio, same protocol as the desktop app.

use std::io::{BufRead, Write};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::Mutex;

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
        Self { config, registry: Arc::new(Mutex::new(registry)) }
    }

    pub fn run_blocking(&self) -> anyhow::Result<()> {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        for line in stdin.lock().lines() {
            let line = line?;
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

    fn dispatch_sync(&self, req: Value) -> Option<String> {
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let params = req.get("params").cloned().unwrap_or(Value::Null);
        let rt = tokio::runtime::Runtime::new().ok()?;
        let result = rt.block_on(self.handle_method(method, params));
        match result {
            Ok(v) => {
                if id == Value::Null { return None; }
                Some(serde_json::to_string(&json!({"jsonrpc": "2.0", "id": id, "result": v})).unwrap_or_else(|_| "{}".into()))
            }
            Err(e) => {
                if id == Value::Null { return None; }
                Some(error_response(id, -32000, &format!("{e}")))
            }
        }
    }

    async fn handle_method(&self, method: &str, params: Value) -> anyhow::Result<Value> {
        match method {
            "initialize" => Ok(json!({
                "protocolVersion": self.config.protocol_version,
                "serverInfo": {
                    "name": self.config.name,
                    "version": self.config.version,
                },
                "capabilities": { "tools": {} }
            })),
            "ping" => Ok(json!({})),
            "tools/list" => {
                let reg = self.registry.lock().await;
                let tools: Vec<Value> = reg.list().iter().map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "inputSchema": {
                            "type": "object",
                            "properties": t.schema.params.iter().filter_map(|p| {
                                Some((p.name.clone(), json!({"type": p.ty, "description": p.description})))
                            }).collect::<serde_json::Map<String, Value>>(),
                            "required": t.schema.params.iter().filter(|p| p.required).map(|p| p.name.clone()).collect::<Vec<_>>(),
                        }
                    })
                }).collect();
                Ok(json!({ "tools": tools }))
            }
            "tools/call" => {
                let name = params.get("name").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("tools/call requires 'name'"))?;
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
            _ => anyhow::bail!("unknown method: {method}"),
        }
    }
}

fn error_response(id: Value, code: i64, message: &str) -> String {
    serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })).unwrap_or_else(|_| "{}".into())
}
