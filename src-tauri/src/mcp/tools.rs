//! v1.7.0 — MCP tool definitions and registry.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::Result;

/// JSON-schema-ish parameter declaration. Intentionally minimal — we
/// only need enough to advertise to MCP clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParam {
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub ty: String, // "string" | "number" | "boolean" | "array" | "object"
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub params: Vec<ToolParam>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub schema: ToolSchema,
}

/// Type-erased handler. The handler receives the parsed arguments object
/// (as a `serde_json::Value`) and returns a `serde_json::Value` result.
pub type Handler =
    Arc<dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send>> + Send + Sync>;

#[derive(Clone)]
pub struct RegisteredTool {
    pub tool: Tool,
    pub handler: Handler,
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: Vec<RegisteredTool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool: Tool, handler: Handler) {
        self.tools.push(RegisteredTool { tool, handler });
    }

    pub fn list(&self) -> Vec<&Tool> {
        self.tools.iter().map(|t| &t.tool).collect()
    }

    pub fn get(&self, name: &str) -> Option<&RegisteredTool> {
        self.tools.iter().find(|t| t.tool.name == name)
    }

    pub async fn call(&self, name: &str, args: Value) -> Result<Value> {
        let t = self
            .get(name)
            .ok_or_else(|| crate::error::Error::Other(format!("unknown tool: {name}")))?;
        (t.handler)(args).await
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

/// Build the default tool registry. The handlers are no-ops returning a
/// short "not wired in standalone mode" message — the desktop app and
/// the CLI both replace these with real implementations at startup.
pub fn registry_default() -> ToolRegistry {
    let mut r = ToolRegistry::new();
    let noop: Handler = Arc::new(|args: Value| {
        Box::pin(async move {
            Ok(serde_json::json!({
                "ok": true,
                "echo": args,
                "note": "default no-op handler — replace at startup"
            }))
        })
    });
    let tools = vec![
        Tool {
            name: "memory_search".into(),
            description: "Search Aegis AI's persistent memory store (conversations, knowledge, embeddings).".into(),
            schema: ToolSchema {
                params: vec![
                    ToolParam { name: "query".into(), description: "Free-text query".into(), ty: "string".into(), required: true },
                    ToolParam { name: "limit".into(), description: "Max results (default 5)".into(), ty: "number".into(), required: false },
                ],
            },
        },
        Tool {
            name: "memory_remember".into(),
            description: "Persist a durable fact into the knowledge base.".into(),
            schema: ToolSchema {
                params: vec![
                    ToolParam { name: "key".into(), description: "Stable lookup key".into(), ty: "string".into(), required: true },
                    ToolParam { name: "value".into(), description: "The fact to remember".into(), ty: "string".into(), required: true },
                    ToolParam { name: "confidence".into(), description: "0..1 (default 0.7)".into(), ty: "number".into(), required: false },
                ],
            },
        },
        Tool {
            name: "skills_match".into(),
            description: "Find published skills whose trigger keywords match the given message.".into(),
            schema: ToolSchema {
                params: vec![
                    ToolParam { name: "message".into(), description: "User message text".into(), ty: "string".into(), required: true },
                ],
            },
        },
        Tool {
            name: "world_news".into(),
            description: "Fetch the latest news briefs from Aegis AI's world intelligence registry.".into(),
            schema: ToolSchema {
                params: vec![
                    ToolParam { name: "category".into(), description: "Optional category filter: world, geopolitics, tech, finance, security, science, disaster".into(), ty: "string".into(), required: false },
                    ToolParam { name: "limit".into(), description: "Max items (default 10)".into(), ty: "number".into(), required: false },
                ],
            },
        },
        Tool {
            name: "world_finance".into(),
            description: "Fetch a market snapshot (stocks, FX, crypto). Symbols: crypto:bitcoin, fx:EURUSD, or ticker like AAPL.".into(),
            schema: ToolSchema {
                params: vec![
                    ToolParam { name: "symbols".into(), description: "Array of symbols".into(), ty: "array".into(), required: false },
                ],
            },
        },
        Tool {
            name: "world_risk".into(),
            description: "Compute the Country Instability Index for a list of countries.".into(),
            schema: ToolSchema {
                params: vec![
                    ToolParam { name: "countries".into(), description: "Array of {iso3, name, news_volume, negative_ratio, disaster_count, market_stress}".into(), ty: "array".into(), required: true },
                ],
            },
        },
        Tool {
            name: "wiki_search".into(),
            description: "Search the local Wiki knowledge base.".into(),
            schema: ToolSchema {
                params: vec![
                    ToolParam { name: "query".into(), description: "Free-text query".into(), ty: "string".into(), required: true },
                ],
            },
        },
        Tool {
            name: "codegraph_search".into(),
            description: "Search indexed code symbols by name.".into(),
            schema: ToolSchema {
                params: vec![
                    ToolParam { name: "name".into(), description: "Symbol name (or substring)".into(), ty: "string".into(), required: true },
                ],
            },
        },
        Tool {
            name: "graph_query".into(),
            description: "Query the knowledge graph by (subject, predicate, object) pattern; any None is a wildcard.".into(),
            schema: ToolSchema {
                params: vec![
                    ToolParam { name: "subject".into(), description: "Optional subject".into(), ty: "string".into(), required: false },
                    ToolParam { name: "predicate".into(), description: "Optional predicate".into(), ty: "string".into(), required: false },
                    ToolParam { name: "object".into(), description: "Optional object".into(), ty: "string".into(), required: false },
                ],
            },
        },
    ];
    for t in tools {
        r.register(t, noop.clone());
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_default_has_all_tools() {
        let r = registry_default();
        assert!(r.len() >= 9);
        assert!(r.get("memory_search").is_some());
        assert!(r.get("world_news").is_some());
        assert!(r.get("graph_query").is_some());
    }

    #[tokio::test]
    async fn noop_handler_returns_echo() {
        let r = registry_default();
        let out = r.call("memory_search", serde_json::json!({"query": "rust"})).await.unwrap();
        assert_eq!(out["ok"], true);
        assert_eq!(out["echo"]["query"], "rust");
    }
}
