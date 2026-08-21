//! MCP tool registry — slim CLI version. Same shape as the desktop app's.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParam {
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub ty: String,
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

pub type Handler = Arc<
    dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<Value>> + Send>>
        + Send + Sync,
>;

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

    pub async fn call(&self, name: &str, args: Value) -> anyhow::Result<Value> {
        let t = self.get(name).ok_or_else(|| anyhow::anyhow!("unknown tool: {name}"))?;
        (t.handler)(args).await
    }

    pub fn len(&self) -> usize { self.tools.len() }
    pub fn is_empty(&self) -> bool { self.tools.is_empty() }
}

/// Default registry with no-op handlers (the desktop app + CLI both replace these at startup).
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
        Tool { name: "memory_search".into(), description: "Search Aegis AI's persistent memory store.".into(), schema: ToolSchema { params: vec![
            ToolParam { name: "query".into(), description: "Free-text query".into(), ty: "string".into(), required: true },
            ToolParam { name: "limit".into(), description: "Max results".into(), ty: "number".into(), required: false },
        ]}},
        Tool { name: "memory_remember".into(), description: "Persist a durable fact.".into(), schema: ToolSchema { params: vec![
            ToolParam { name: "key".into(), description: "Stable key".into(), ty: "string".into(), required: true },
            ToolParam { name: "value".into(), description: "Fact to remember".into(), ty: "string".into(), required: true },
        ]}},
        Tool { name: "skills_match".into(), description: "Find published skills whose triggers match a message.".into(), schema: ToolSchema { params: vec![
            ToolParam { name: "message".into(), description: "User message".into(), ty: "string".into(), required: true },
        ]}},
        Tool { name: "world_news".into(), description: "Fetch latest news briefs.".into(), schema: ToolSchema { params: vec![
            ToolParam { name: "category".into(), description: "Optional filter".into(), ty: "string".into(), required: false },
            ToolParam { name: "limit".into(), description: "Max items".into(), ty: "number".into(), required: false },
        ]}},
        Tool { name: "world_finance".into(), description: "Fetch market quotes.".into(), schema: ToolSchema { params: vec![
            ToolParam { name: "symbols".into(), description: "Symbol array".into(), ty: "array".into(), required: false },
        ]}},
        Tool { name: "world_risk".into(), description: "Compute country instability index.".into(), schema: ToolSchema { params: vec![
            ToolParam { name: "countries".into(), description: "Country array".into(), ty: "array".into(), required: true },
        ]}},
        Tool { name: "wiki_search".into(), description: "Search the local Wiki knowledge base.".into(), schema: ToolSchema { params: vec![
            ToolParam { name: "query".into(), description: "Query".into(), ty: "string".into(), required: true },
        ]}},
        Tool { name: "codegraph_search".into(), description: "Search indexed code symbols by name.".into(), schema: ToolSchema { params: vec![
            ToolParam { name: "name".into(), description: "Symbol name".into(), ty: "string".into(), required: true },
        ]}},
        Tool { name: "graph_query".into(), description: "Query the knowledge graph (s,p,o) pattern.".into(), schema: ToolSchema { params: vec![
            ToolParam { name: "subject".into(), description: "Optional subject".into(), ty: "string".into(), required: false },
            ToolParam { name: "predicate".into(), description: "Optional predicate".into(), ty: "string".into(), required: false },
            ToolParam { name: "object".into(), description: "Optional object".into(), ty: "string".into(), required: false },
        ]}},
    ];
    for t in tools {
        r.register(t, noop.clone());
    }
    r
}
