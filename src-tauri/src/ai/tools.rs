//! Tool registry for the AI agent loop.
//!
//! Defines the schema for each computer-use tool the AI may invoke, plus the
//! dispatcher that runs a tool call locally and returns its result.
//!
//! ## Tools available to the AI in v0.3
//!
//! | Tool              | Purpose                                    | Risk     |
//! |-------------------|--------------------------------------------|----------|
//! | `shell`           | Run a shell command (gated by safety).     | medium   |
//! | `file_read`       | Read up to 1 MB of a file.                 | safe     |
//! | `file_write`      | Write text to a file (gated by safety).    | medium   |
//! | `file_list`       | List a directory's contents.               | safe     |
//! | `app_open`        | Launch an app by name.                     | low      |
//! | `app_list`        | Enumerate installed apps.                  | safe     |
//! | `screenshot`      | Capture the primary screen + OCR.          | safe     |
//! | `gui_action`      | Move/click/scroll/type via enigo.           | low      |
//! | `clipboard_read`  | Read clipboard text.                       | safe     |
//! | `clipboard_write`  | Write clipboard text.                      | low      |
//! | `web_search`      | (Stubbed) Web search stub for v0.3.         | safe     |
//! | `web_fetch`       | (Stubbed) Fetch a URL's content.          | low      |
//! | `memory_remember` | Store a key/value fact in the knowledge base. | safe  |
//! | `memory_lookup`   | Retrieve a stored fact.                    | safe     |
//!
//! All tools run through the safety policy where applicable. The agent loop
//! (`ai::agent`) is the only caller — it never exposes raw tool dispatch to
//! the frontend.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::ai::provider::ChatMessage;
use crate::computer::{
    apps::{list_apps, open_app, AppDescriptor},
    automation::{auto_perform, AutoAction},
    clipboard::{clipboard_read, clipboard_write, ClipboardContent},
    commands::ExecResult,
    files::{file_read, FileReadResult},
    safety::{SafetyDecision, SafetyPolicy},
    screenshot,
};
use crate::error::{AegisError, Result};
use crate::memory::KnowledgeEntry;

/// JSON schema fragment advertising a single tool to the AI.
/// We use the OpenAI "tools" array shape so it works with any OpenAI-compat
/// provider that supports function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub r#type: String,
    pub function: FunctionSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSpec {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// A single tool call requested by the AI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// Arguments parsed as JSON (or the raw string if parsing fails).
    pub arguments: Value,
}

/// The outcome of executing a tool.
#[derive(Debug, Clone, Serialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub name: String,
    /// Stringified result content (added to the assistant's context as a `tool` message).
    pub content: String,
    /// True if the tool ran successfully; false if it errored or was denied.
    pub success: bool,
}

/// Return the list of all available tool specs (sent to the AI on each call).
pub fn all_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "shell".into(),
                description: "Execute a shell command on the user's machine. The command is gated by the safety policy — destructive or unwhitelisted commands require explicit user confirmation.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "command": {"type": "string", "description": "The shell command to execute, e.g. `ls -la` or `git status`."}
                    },
                    "required": ["command"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "file_read".into(),
                description: "Read the contents of a file. Returns up to 1 MB; larger files are truncated.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Absolute or tilde-expanded path to the file."}
                    },
                    "required": ["path"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "file_write".into(),
                description: "Write text content to a file. Creates parent directories. Gated by the safety policy: writes outside the user-approved whitelist require confirmation.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "content": {"type": "string"}
                    },
                    "required": ["path", "content"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "file_list".into(),
                description: "List the files and directories inside a directory.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"}
                    },
                    "required": ["path"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "app_open".into(),
                description: "Launch an installed application by name (e.g. `notepad`, `code`).".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    },
                    "required": ["name"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "app_list".into(),
                description: "List installed applications discoverable on the user's PATH and desktop.".into(),
                parameters: json!({"type": "object", "properties": {}}),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "screenshot".into(),
                description: "Capture the primary screen. Returns the image dimensions, base64 PNG, and OCR-extracted text.".into(),
                parameters: json!({"type": "object", "properties": {}}),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "gui_action".into(),
                description: "Perform one or more GUI automation actions (mouse move/click/scroll, type text, press key combos). Use sparingly — prefer `shell` for programmatic tasks.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "actions": {
                            "type": "array",
                            "description": "List of declarative GUI actions; see AutoAction enum.",
                            "items": {"type": "object"}
                        }
                    },
                    "required": ["actions"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "clipboard_read".into(),
                description: "Read the current text content of the clipboard.".into(),
                parameters: json!({"type": "object", "properties": {}}),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "clipboard_write".into(),
                description: "Write text to the clipboard.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "text": {"type": "string"}
                    },
                    "required": ["text"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "web_search".into(),
                description: "Search the web for a query. (Stubbed in v0.3 — returns an empty result list with a note.)".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    },
                    "required": ["query"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "memory_remember".into(),
                description: "Persist a key/value fact in the Aegis knowledge base so it can be recalled later.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "key":   {"type": "string"},
                        "value": {"type": "string"}
                    },
                    "required": ["key", "value"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "memory_lookup".into(),
                description: "Recall a previously stored fact from the knowledge base.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "key": {"type": "string"}
                    },
                    "required": ["key"]
                }),
            },
        },
    ]
}

/// Convert tool specs into the JSON value expected in the OpenAI `tools` field.
pub fn specs_as_json() -> Value {
    serde_json::to_value(all_specs()).unwrap_or_else(|_| Value::Array(vec![]))
}

/// Execute a single tool call locally, returning a `ToolResult` to feed back
/// into the conversation.
///
/// Safety: every tool that mutates state routes through `policy`. If the
/// policy returns `RequireConfirmation`, we surface that as a `success=false`
/// ToolResult containing the confirmation token so the AI can ask the user.
pub fn dispatch(
    call: &ToolCall,
    policy: &SafetyPolicy,
    memory: &std::sync::Arc<crate::memory::store::MemoryStore>,
) -> ToolResult {
    let content = match call.name.as_str() {
        "shell" => run_shell(policy, &call.arguments),
        "file_read" => run_file_read(&call.arguments),
        "file_write" => run_file_write(policy, &call.arguments),
        "file_list" => run_file_list(&call.arguments),
        "app_open" => run_app_open(policy, &call.arguments),
        "app_list" => serde_json::to_string(&list_apps()).unwrap_or_else(|_| "[]".into()),
        "screenshot" => match screenshot() {
            Ok(s) => serde_json::to_string(&s).unwrap_or_else(|_| "{}".into()),
            Err(e) => error_json(&call.name, &e),
        },
        "gui_action" => run_gui_action(&call.arguments),
        "clipboard_read" => match clipboard_read() {
            Ok(c) => serde_json::to_string(&c).unwrap_or_else(|_| "{}".into()),
            Err(e) => error_json(&call.name, &e),
        },
        "clipboard_write" => run_clipboard_write(&call.arguments),
        "web_search" => json!({
            "results": [],
            "note": "web_search is stubbed in v0.3 — wire up your favourite search provider to enable this."
        })
        .to_string(),
        "memory_remember" => run_memory_remember(memory, &call.arguments),
        "memory_lookup" => run_memory_lookup(memory, &call.arguments),
        other => format!("{{\"error\":\"unknown tool '{other}'\"}}"),
    };

    let success = !content.contains("\"error\":")
        && !content.contains("\"safety_decision\":\"require_confirmation\"")
        && !content.contains("\"safety_decision\":\"deny\"");

    ToolResult {
        tool_call_id: call.id.clone(),
        name: call.name.clone(),
        content,
        success,
    }
}

fn run_shell(policy: &SafetyPolicy, args: &Value) -> String {
    let cmd = match args.get("command").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'command' argument"}).to_string(),
    };
    match policy.check_command(&cmd) {
        SafetyDecision::Allow => match crate::computer::commands::exec_command_authorized(&cmd) {
            Ok(r) => serde_json::to_string(&r).unwrap_or_else(|_| "{}".into()),
            Err(e) => error_json("shell", &e),
        },
        SafetyDecision::Deny { reason } => json!({
            "safety_decision": "deny",
            "reason": reason,
            "command": cmd,
        })
        .to_string(),
        SafetyDecision::RequireConfirmation { token, summary, .. } => json!({
            "safety_decision": "require_confirmation",
            "token": token,
            "summary": summary,
            "command": cmd,
            "hint": "Tell the user to approve this action in the Aegis UI."
        })
        .to_string(),
    }
}

fn run_file_read(args: &Value) -> String {
    let path = match args.get("path").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'path' argument"}).to_string(),
    };
    match file_read(&path) {
        Ok(r) => serde_json::to_string(&r).unwrap_or_else(|_| "{}".into()),
        Err(e) => error_json("file_read", &e),
    }
}

fn run_file_write(policy: &SafetyPolicy, args: &Value) -> String {
    let path = match args.get("path").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'path' argument"}).to_string(),
    };
    let content = match args.get("content").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'content' argument"}).to_string(),
    };
    match policy.check_file_write(&path) {
        SafetyDecision::Allow => match crate::computer::files::file_write_authorized(&path, &content) {
            Ok(_) => json!({"ok": true, "path": path, "bytes": content.len()}).to_string(),
            Err(e) => error_json("file_write", &e),
        },
        SafetyDecision::Deny { reason } => json!({
            "safety_decision": "deny",
            "reason": reason,
            "path": path,
        })
        .to_string(),
        SafetyDecision::RequireConfirmation { token, summary, .. } => json!({
            "safety_decision": "require_confirmation",
            "token": token,
            "summary": summary,
            "path": path,
        })
        .to_string(),
    }
}

fn run_file_list(args: &Value) -> String {
    let path = match args.get("path").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'path' argument"}).to_string(),
    };
    match std::fs::read_dir(&path) {
        Ok(iter) => {
            let entries: Vec<String> = iter
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            json!({"path": path, "entries": entries}).to_string()
        }
        Err(e) => error_json("file_list", &AegisError::Io(e.to_string())),
    }
}

fn run_app_open(policy: &SafetyPolicy, args: &Value) -> String {
    let name = match args.get("name").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'name' argument"}).to_string(),
    };
    match policy.check_app_launch(&name) {
        SafetyDecision::Allow => match open_app_allow(&name) {
            Ok(_) => json!({"ok": true, "app": name}).to_string(),
            Err(e) => error_json("app_open", &e),
        },
        SafetyDecision::Deny { reason } => json!({
            "safety_decision": "deny",
            "reason": reason,
        })
        .to_string(),
        SafetyDecision::RequireConfirmation { token, summary, .. } => json!({
            "safety_decision": "require_confirmation",
            "token": token,
            "summary": summary,
        })
        .to_string(),
    }
}

// Tiny adapter so we don't need to import open_app_authorized through safety.
fn open_app_allow(name: &str) -> Result<()> {
    if let Err(e) = crate::computer::apps::open_app_authorized(name) {
        return Err(e);
    }
    Ok(())
}

fn run_gui_action(args: &Value) -> String {
    let actions_val = match args.get("actions") {
        Some(v) => v,
        None => return json!({"error":"missing 'actions' array"}).to_string(),
    };
    let actions: Vec<AutoAction> = match serde_json::from_value(actions_val.clone()) {
        Ok(v) => v,
        Err(e) => return json!({"error":"failed to parse actions: {e}"}).to_string(),
    };
    match auto_perform(actions) {
        Ok(_) => json!({"ok": true}).to_string(),
        Err(e) => error_json("gui_action", &e),
    }
}

fn run_clipboard_write(args: &Value) -> String {
    let text = match args.get("text").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'text' argument"}).to_string(),
    };
    match clipboard_write(&text) {
        Ok(_) => json!({"ok": true}).to_string(),
        Err(e) => error_json("clipboard_write", &e),
    }
}

fn run_memory_remember(
    memory: &std::sync::Arc<crate::memory::store::MemoryStore>,
    args: &Value,
) -> String {
    let key = match args.get("key").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'key' argument"}).to_string(),
    };
    let value = match args.get("value").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'value' argument"}).to_string(),
    };
    match memory.knowledge.remember(&key, &value, Some("agent"), 0.8) {
        Ok(_) => json!({"ok": true, "key": key}).to_string(),
        Err(e) => error_json("memory_remember", &e),
    }
}

fn run_memory_lookup(
    memory: &std::sync::Arc<crate::memory::store::MemoryStore>,
    args: &Value,
) -> String {
    let key = match args.get("key").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'key' argument"}).to_string(),
    };
    match memory.knowledge.lookup(&key) {
        Ok(Some(entry)) => serde_json::to_string(&entry).unwrap_or_else(|_| "{}".into()),
        Ok(None) => json!({"found": false, "key": key}).to_string(),
        Err(e) => error_json("memory_lookup", &e),
    }
}

fn error_json(tool: &str, e: &AegisError) -> String {
    json!({
        "error": e.to_string(),
        "tool": tool,
    })
    .to_string()
}

/// Convert a `ToolResult` into the `ChatMessage` we feed back to the AI
/// (role=tool, content=the result JSON, name=tool name).
pub fn result_to_message(r: &ToolResult) -> ChatMessage {
    ChatMessage {
        role: crate::ai::provider::Role::Tool,
        content: r.content.clone(),
        name: Some(r.name.clone()),
        tool_calls: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specs_are_nonempty() {
        let s = all_specs();
        assert!(s.len() >= 12);
        // Each spec has a name and a parameters object.
        for spec in &s {
            assert!(!spec.function.name.is_empty());
            assert!(spec.function.parameters.is_object());
        }
    }

    #[test]
    fn specs_serialize_for_openai() {
        let v = specs_as_json();
        assert!(v.is_array());
        assert!(v.as_array().unwrap().len() >= 12);
    }
}
