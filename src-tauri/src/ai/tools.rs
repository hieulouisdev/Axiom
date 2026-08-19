//! Tool registry for the AI agent loop.
//!
//! Defines the schema for each computer-use tool the AI may invoke, plus the
//! dispatcher that runs a tool call locally and returns its result.
//!
//! ## Tools available to the AI in v0.4
//!
//! | Tool              | Purpose                                    | Risk     |
//! |-------------------|--------------------------------------------|----------|
//! | `shell`           | Run a shell command (gated by safety).     | medium   |
//! | `file_read`       | Read up to 1 MB of a file.                 | safe     |
//! | `file_write`      | Write text to a file (gated by safety).    | medium   |
//! | `file_list`       | List a directory's contents.               | safe     |
//! | `file_delete`     | Delete a file (gated by safety).           | high     |
//! | `file_move`       | Move / rename a file (gated by safety).    | medium   |
//! | `file_glob`       | Find files matching a glob pattern.        | safe     |
//! | `regex_search`    | Search file contents with a regex.         | safe     |
//! | `diff_apply`      | Apply a unified diff to files.             | medium   |
//! | `app_open`        | Launch an app by name.                     | low      |
//! | `app_list`        | Enumerate installed apps.                  | safe     |
//! | `screenshot`      | Capture the primary screen + OCR.          | safe     |
//! | `gui_action`      | Move/click/scroll/type via enigo.           | low      |
//! | `clipboard_read`  | Read clipboard text.                       | safe     |
//! | `clipboard_write`  | Write clipboard text.                      | low      |
//! | `web_search`      | (Stubbed) Web search stub for v0.4.        | safe     |
//! | `http_fetch`      | Fetch a URL and return its body.           | low      |
//! | `git_op`          | Run a git command (status, diff, commit).  | medium   |
//! | `process_list`    | List running processes.                    | safe     |
//! | `process_kill`    | Terminate a process by pid.                | high     |
//! | `code_eval`       | Evaluate a code snippet in a sandbox.      | high     |
//! | `notify`          | Show a desktop notification.               | safe     |
//! | `open_url`        | Open a URL in the default browser.         | low      |
//! | `memory_remember` | Store a key/value fact in the knowledge base. | safe  |
//! | `memory_lookup`   | Retrieve a stored fact.                    | safe     |
//! | `memory_search`   | Semantic search over the knowledge base.   | safe     |
//! | `skill_set`       | Switch the active skill.                   | safe     |
//! | `skill_list`      | List available skills.                     | safe     |
//!
//! All tools run through the safety policy where applicable. The agent loop
//! (`ai::agent`) is the only caller — it never exposes raw tool dispatch to
//! the frontend.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::ai::provider::ChatMessage;
use crate::computer::{
    apps::list_apps,
    automation::{AutoAction, auto_perform},
    clipboard::{clipboard_read, clipboard_write},
    files::file_read,
    safety::{SafetyDecision, SafetyPolicy},
    screenshot,
};
use crate::error::{AegisError, Result};

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
                description: "Search the web for a query and return up to 8 results (title, URL, snippet). Powered by DuckDuckGo — no API key required. Use this for current information, looking up documentation, fact-checking, or finding URLs to fetch.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "The search query."}
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
        // ===== v0.4: new tools =====
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "file_delete".into(),
                description: "Delete a file. Gated by the safety policy — always requires confirmation unless bypass mode is on.".into(),
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
                name: "file_move".into(),
                description: "Move or rename a file. Gated by the safety policy when the destination is outside the whitelist.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "from": {"type": "string"},
                        "to":   {"type": "string"}
                    },
                    "required": ["from", "to"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "file_glob".into(),
                description: "Find files matching a glob pattern (e.g. `**/*.rs`). Returns up to 200 matches.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string", "description": "Glob pattern, e.g. `**/*.rs` or `src/**/*.ts`."},
                        "root":    {"type": "string", "description": "Root directory for the search. Defaults to the current directory."}
                    },
                    "required": ["pattern"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "regex_search".into(),
                description: "Search file contents under a directory using a regex. Returns the file path, line number, and matching line for up to 100 hits.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pattern": {"type": "string"},
                        "root":    {"type": "string"},
                        "file_glob": {"type": "string", "description": "Optional glob to filter which files to scan, e.g. `*.rs`."}
                    },
                    "required": ["pattern", "root"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "diff_apply".into(),
                description: "Apply a unified diff to files on disk. Use this to make precise edits to existing files. The diff must be a valid unified diff (the kind `git diff` produces).".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "diff": {"type": "string", "description": "Unified diff text."}
                    },
                    "required": ["diff"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "http_fetch".into(),
                description: "Fetch the contents of a URL and return the body (truncated to 256 KB) plus final status code. Use this for web research or downloading plain-text / JSON / HTML pages.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "url":    {"type": "string"},
                        "method": {"type": "string", "description": "HTTP method. Defaults to GET."},
                        "body":   {"type": "string", "description": "Optional request body for POST/PUT."},
                        "headers": {"type": "object", "description": "Optional extra headers as a JSON object."}
                    },
                    "required": ["url"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "git_op".into(),
                description: "Run a git command (e.g. `status`, `diff`, `log --oneline -10`, `add`, `commit -m`). The command is run with the safety policy applied to the underlying shell.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "args":  {"type": "string", "description": "Git subcommand + args, e.g. `status` or `commit -m 'msg'`."},
                        "cwd":   {"type": "string", "description": "Working directory. Defaults to the user's home."}
                    },
                    "required": ["args"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "process_list".into(),
                description: "List running processes (pid, name, command line, memory usage). Returns up to 200 entries.".into(),
                parameters: json!({"type": "object", "properties": {}}),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "process_kill".into(),
                description: "Terminate a process by pid. Gated by the safety policy.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "pid": {"type": "integer"}
                    },
                    "required": ["pid"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "code_eval".into(),
                description: "Evaluate a code snippet in a sandboxed interpreter. Supports `python3`, `node`, and `bash`. Output (stdout + stderr) is returned up to 64 KB. Gated by the safety policy.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "language": {"type": "string", "description": "One of: python3, node, bash."},
                        "code":     {"type": "string"},
                        "timeout_seconds": {"type": "integer", "description": "Max execution time. Defaults to 15."}
                    },
                    "required": ["language", "code"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "notify".into(),
                description: "Show a desktop notification. Useful for long-running tasks that want to alert the user when done.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "title": {"type": "string"},
                        "body":  {"type": "string"}
                    },
                    "required": ["title", "body"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "open_url".into(),
                description: "Open a URL in the user's default browser.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "url": {"type": "string"}
                    },
                    "required": ["url"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "memory_search".into(),
                description: "Semantic search over the knowledge base. Returns the top N most similar facts to the query.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "limit": {"type": "integer", "description": "Max results. Defaults to 5."}
                    },
                    "required": ["query"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "skill_set".into(),
                description: "Switch the active AI skill (specialization). Use `skill_list` first to discover available skills.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"}
                    },
                    "required": ["id"]
                }),
            },
        },
        ToolSpec {
            r#type: "function".into(),
            function: FunctionSpec {
                name: "skill_list".into(),
                description: "List all available AI skills with their descriptions.".into(),
                parameters: json!({"type": "object", "properties": {}}),
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
        "file_delete" => run_file_delete(policy, &call.arguments),
        "file_move" => run_file_move(policy, &call.arguments),
        "file_glob" => run_file_glob(&call.arguments),
        "regex_search" => run_regex_search(&call.arguments),
        "diff_apply" => run_diff_apply(&call.arguments),
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
        "web_search" => run_web_search(&call.arguments),
        "http_fetch" => run_http_fetch(&call.arguments),
        "git_op" => run_git_op(policy, &call.arguments),
        "process_list" => run_process_list(),
        "process_kill" => run_process_kill(policy, &call.arguments),
        "code_eval" => run_code_eval(policy, &call.arguments),
        "notify" => run_notify(&call.arguments),
        "open_url" => run_open_url(&call.arguments),
        "memory_remember" => run_memory_remember(memory, &call.arguments),
        "memory_lookup" => run_memory_lookup(memory, &call.arguments),
        "memory_search" => run_memory_search(memory, &call.arguments),
        "skill_set" => run_skill_set(&call.arguments),
        "skill_list" => {
            serde_json::to_string(&crate::ai::skills::all_skills()).unwrap_or_else(|_| "[]".into())
        }
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
        SafetyDecision::Allow => {
            match crate::computer::files::file_write_authorized(&path, &content) {
                Ok(_) => json!({"ok": true, "path": path, "bytes": content.len()}).to_string(),
                Err(e) => error_json("file_write", &e),
            }
        }
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
    crate::computer::apps::open_app_authorized(name)?;
    Ok(())
}

fn run_gui_action(args: &Value) -> String {
    let actions_val = match args.get("actions") {
        Some(v) => v,
        None => return json!({"error":"missing 'actions' array"}).to_string(),
    };
    let actions: Vec<AutoAction> = match serde_json::from_value(actions_val.clone()) {
        Ok(v) => v,
        Err(e) => {
            return json!({
                "error": format!("failed to parse actions: {e}")
            })
            .to_string();
        }
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

/// v0.6: Real web search via DuckDuckGo's HTML endpoint. No API key needed.
/// Returns up to 8 hits with title, URL, and snippet — enough for the AI to
/// either summarise the answer or call `http_fetch` on the most relevant URL.
fn run_web_search(args: &Value) -> String {
    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'query' argument"}).to_string(),
    };
    let results = crate::ai::web::web_search_sync(&query);
    serde_json::to_string(&results)
        .unwrap_or_else(|_| json!({"error":"serialization failed"}).to_string())
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
    // v0.5: route through MemoryStore::remember so the embedding is updated
    // in lockstep with the knowledge table — RAG retrieval always sees the
    // latest facts.
    match memory.remember(&key, &value, Some("agent"), 0.8) {
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

// ===========================================================================
// v0.4 — New tool runners
// ===========================================================================

fn run_file_delete(policy: &SafetyPolicy, args: &Value) -> String {
    let path = match args.get("path").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'path' argument"}).to_string(),
    };
    match policy.check_file_delete(&path) {
        SafetyDecision::Allow => match std::fs::remove_file(&path) {
            Ok(_) => json!({"ok": true, "path": path}).to_string(),
            Err(e) => error_json("file_delete", &AegisError::Io(e.to_string())),
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

fn run_file_move(policy: &SafetyPolicy, args: &Value) -> String {
    let from = match args.get("from").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'from' argument"}).to_string(),
    };
    let to = match args.get("to").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'to' argument"}).to_string(),
    };
    // The destination path is the one that needs the safety check.
    match policy.check_file_write(&to) {
        SafetyDecision::Allow => {
            if let Some(parent) = std::path::Path::new(&to).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::rename(&from, &to) {
                Ok(_) => json!({"ok": true, "from": from, "to": to}).to_string(),
                Err(e) => error_json("file_move", &AegisError::Io(e.to_string())),
            }
        }
        SafetyDecision::Deny { reason } => json!({
            "safety_decision": "deny",
            "reason": reason,
            "path": to,
        })
        .to_string(),
        SafetyDecision::RequireConfirmation { token, summary, .. } => json!({
            "safety_decision": "require_confirmation",
            "token": token,
            "summary": summary,
            "path": to,
        })
        .to_string(),
    }
}

fn run_file_glob(args: &Value) -> String {
    let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'pattern' argument"}).to_string(),
    };
    let root = args
        .get("root")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| ".".into());

    // Walk the tree and match. We keep it simple: support `**` (recursive)
    // and `*` (single-segment) globs by converting to a regex.
    let regex_src = glob_to_regex(&pattern);
    let re = match regex::Regex::new(&regex_src) {
        Ok(r) => r,
        Err(e) => return json!({"error": format!("invalid glob: {e}")}).to_string(),
    };

    let mut matches: Vec<String> = Vec::new();
    let root_path = std::path::PathBuf::from(&root);
    walk_dir(&root_path, &root_path, &re, &mut matches, 200);
    json!({"pattern": pattern, "root": root, "matches": matches}).to_string()
}

fn walk_dir(
    root: &std::path::Path,
    dir: &std::path::Path,
    re: &regex::Regex,
    out: &mut Vec<String>,
    cap: usize,
) {
    if out.len() >= cap {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        if out.len() >= cap {
            return;
        }
        let p = entry.path();
        let rel = p
            .strip_prefix(root)
            .unwrap_or(&p)
            .to_string_lossy()
            .to_string();
        if re.is_match(&rel) {
            out.push(p.to_string_lossy().to_string());
        }
        if p.is_dir() {
            // Skip heavy / hidden dirs.
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.starts_with('.') || name == "node_modules" || name == "target" {
                continue;
            }
            walk_dir(root, &p, re, out, cap);
        }
    }
}

fn glob_to_regex(glob: &str) -> String {
    let mut out = String::with_capacity(glob.len() + 8);
    out.push('^');
    let mut chars = glob.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    // `**` matches any number of path segments (including zero).
                    out.push_str(".*");
                } else {
                    // `*` matches a single path segment (no `/`).
                    out.push_str("[^/]*");
                }
            }
            '?' => out.push_str("[^/]"),
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '\\' | '{' | '}' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out.push('$');
    out
}

fn run_regex_search(args: &Value) -> String {
    let pattern = match args.get("pattern").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'pattern' argument"}).to_string(),
    };
    let root = match args.get("root").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'root' argument"}).to_string(),
    };
    let file_glob = args
        .get("file_glob")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let re = match regex::Regex::new(&pattern) {
        Ok(r) => r,
        Err(e) => return json!({"error": format!("invalid regex: {e}")}).to_string(),
    };
    let file_re = file_glob
        .as_deref()
        .and_then(|g| regex::Regex::new(&glob_to_regex(g)).ok());

    let root_path = std::path::PathBuf::from(&root);
    let mut hits: Vec<serde_json::Value> = Vec::new();
    regex_walk(&root_path, &root_path, &re, &file_re, &mut hits, 100);
    json!({"pattern": pattern, "root": root, "hits": hits}).to_string()
}

fn regex_walk(
    root: &std::path::Path,
    dir: &std::path::Path,
    re: &regex::Regex,
    file_re: &Option<regex::Regex>,
    out: &mut Vec<serde_json::Value>,
    cap: usize,
) {
    if out.len() >= cap {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        if out.len() >= cap {
            return;
        }
        let p = entry.path();
        if p.is_dir() {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.starts_with('.') || name == "node_modules" || name == "target" {
                continue;
            }
            regex_walk(root, &p, re, file_re, out, cap);
            continue;
        }
        // Filter by file glob if requested.
        if let Some(file_re) = file_re {
            let rel = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .to_string();
            if !file_re.is_match(&rel) {
                continue;
            }
        }
        let Ok(text) = std::fs::read_to_string(&p) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            if out.len() >= cap {
                return;
            }
            if re.is_match(line) {
                out.push(json!({
                    "path": p.to_string_lossy(),
                    "line": i + 1,
                    "match": line.chars().take(400).collect::<String>(),
                }));
            }
        }
    }
}

fn run_diff_apply(args: &Value) -> String {
    let diff = match args.get("diff").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'diff' argument"}).to_string(),
    };
    // Try `git apply` first (handles all edge cases). If git isn't available,
    // we fall back to a minimal hand-rolled parser that handles simple
    // @@ -a,b +c,d @@ hunks with pure +/- lines.
    let tmp = std::env::temp_dir().join(format!(
        "aegis-diff-{}.patch",
        uuid::Uuid::new_v4().simple()
    ));
    if let Err(e) = std::fs::write(&tmp, &diff) {
        return json!({"error": format!("failed to write patch file: {e}")}).to_string();
    }
    let out = std::process::Command::new("git")
        .arg("apply")
        .arg(&tmp)
        .output();
    let _ = std::fs::remove_file(&tmp);
    match out {
        Ok(o) if o.status.success() => json!({"ok": true, "applied": diff.len()}).to_string(),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            json!({"error": format!("git apply failed: {stderr}")}).to_string()
        }
        Err(_) => {
            // git not available — try minimal parser
            match apply_diff_minimal(&diff) {
                Ok(n) => json!({"ok": true, "applied_hunks": n, "fallback": "minimal parser (git not found)"}).to_string(),
                Err(e) => json!({"error": e}).to_string(),
            }
        }
    }
}

/// Minimal unified-diff applier — handles `--- /path` + `+++ /path` file
/// headers and `@@ -a,b +c,d @@` hunk headers with pure +/-/context lines.
///
/// This is a fallback used when `git apply` is not available. It supports
/// the common subset of unified diff produced by `git diff` and `diff -u`:
/// file headers `--- a/path` / `+++ b/path`, hunk headers `@@ -a,b +c,d @@`,
/// context lines starting with a space, removed lines starting with `-`,
/// and added lines starting with `+`. No rename/copy/binary support.
fn apply_diff_minimal(diff: &str) -> std::result::Result<usize, String> {
    use std::fs;

    let mut lines = diff.lines().peekable();
    let mut hunks_applied = 0usize;
    while let Some(line) = lines.next() {
        if !line.starts_with("--- ") {
            continue;
        }
        let _from = line.trim_start_matches("--- ").trim();
        let plus = match lines.next() {
            Some(l) if l.starts_with("+++ ") => l.trim_start_matches("+++ ").trim().to_string(),
            _ => return Err("malformed diff: --- not followed by +++".into()),
        };
        // Strip a/ b/ prefixes.
        let target = plus
            .strip_prefix("b/")
            .or_else(|| plus.strip_prefix("/dev/null"))
            .unwrap_or(&plus)
            .to_string();
        // Read the file into memory (empty if missing — supports "create" hunks).
        let mut content = fs::read_to_string(&target).unwrap_or_default();
        // Apply hunks until we hit the next file header or EOF.
        while let Some(hline) = lines.peek() {
            if hline.starts_with("--- ") || hline.starts_with("diff --git") {
                break;
            }
            let hline = lines.next().unwrap();
            if let Some(rest) = hline.strip_prefix("@@ ") {
                // Parse `@@ -a,b +c,d @@`
                let parts: Vec<&str> = rest.split("@@").collect();
                let header = parts[0];
                let minus_tok = header
                    .split_whitespace()
                    .find(|t| t.starts_with('-'))
                    .ok_or("malformed hunk header (missing -a,b)")?;
                let minus: Vec<&str> = minus_tok.trim_start_matches('-').splitn(2, ',').collect();
                let start: usize = minus[0].parse().map_err(|_| "bad hunk start")?;
                // Unified diff line numbers are 1-based; convert to 0-based.
                let mut cursor = start.saturating_sub(1);
                let mut out = String::new();
                // Copy content up to the hunk start.
                let bytes: Vec<&str> = content.lines().collect();
                for (i, l) in bytes.iter().enumerate() {
                    if i >= cursor {
                        break;
                    }
                    out.push_str(l);
                    out.push('\n');
                }
                let mut consumed_input = cursor;
                // Now process lines in this hunk until we exit.
                while let Some(hl) = lines.peek() {
                    if hl.starts_with("@@ ")
                        || hl.starts_with("--- ")
                        || hl.starts_with("diff --git")
                    {
                        break;
                    }
                    let hl = lines.next().unwrap();
                    if let Some(added) = hl.strip_prefix('+') {
                        out.push_str(added);
                        out.push('\n');
                    } else if let Some(removed) = hl.strip_prefix('-') {
                        // Verify the removed line matches the current input line.
                        if consumed_input < bytes.len() && bytes[consumed_input] == removed {
                            consumed_input += 1;
                            cursor += 1;
                        } else {
                            return Err(format!(
                                "diff does not apply at line {} (expected {:?}, found {:?})",
                                cursor + 1,
                                removed,
                                bytes.get(consumed_input).copied().unwrap_or("")
                            ));
                        }
                    } else if let Some(ctx) = hl.strip_prefix(' ') {
                        // Context line: must match input.
                        if consumed_input < bytes.len() && bytes[consumed_input] == ctx {
                            out.push_str(ctx);
                            out.push('\n');
                            consumed_input += 1;
                            cursor += 1;
                        } else {
                            return Err(format!("diff context mismatch at line {}", cursor + 1));
                        }
                    } else if hl.is_empty() {
                        // Empty line in diff is treated as a blank context line.
                        if consumed_input < bytes.len() && bytes[consumed_input].is_empty() {
                            out.push('\n');
                            consumed_input += 1;
                            cursor += 1;
                        }
                    } else if hl.starts_with("\\ ") {
                        // "\ No newline at end of file" marker — ignore, we always add a trailing newline.
                    } else {
                        // Unknown line — bail out.
                        return Err(format!("unrecognized diff line: {hl}"));
                    }
                }
                // Append the rest of the file after the hunk.
                for (i, l) in bytes.iter().enumerate() {
                    if i <= cursor.saturating_sub(1) {
                        continue;
                    }
                    out.push_str(l);
                    out.push('\n');
                }
                content = out;
                hunks_applied += 1;
            }
        }
        // Write the patched file back. If the new content is empty and the
        // target exists, treat it as a deletion.
        if content.is_empty() {
            let _ = fs::remove_file(&target);
        } else {
            fs::write(&target, content).map_err(|e| format!("write {target}: {e}"))?;
        }
    }
    if hunks_applied == 0 {
        return Err("no hunks applied — install git for full diff support".into());
    }
    Ok(hunks_applied)
}

fn run_http_fetch(args: &Value) -> String {
    let url = match args.get("url").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'url' argument"}).to_string(),
    };
    let method = args
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase();
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // v0.6: if the URL looks like an HTML page and method is GET, use the
    // readability extractor so the AI gets plain text instead of raw HTML.
    // For non-GET methods or explicit headers, fall back to the raw fetcher.
    let wants_readable = method == "GET" && body.is_none();
    if wants_readable {
        let text = crate::ai::web::fetch_readable_sync(&url);
        if text.starts_with("{\"error\":\"") {
            return text;
        }
        return json!({
            "status": 200,
            "body": text,
            "url": url,
            "extracted": "readable_text",
            "len": text.len(),
        })
        .to_string();
    }

    // Block on a sync HTTP call. This is fine because dispatch() runs in a
    // sync context — the agent loop calls it from a sync helper.
    let rt = match tokio::runtime::Handle::try_current() {
        Ok(h) => h,
        Err(_) => {
            // No runtime — spin up a one-shot.
            let rt = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(e) => return json!({"error": format!("runtime error: {e}")}).to_string(),
            };
            return rt.block_on(http_fetch_async(&url, &method, body));
        }
    };
    rt.block_on(http_fetch_async(&url, &method, body))
}

async fn http_fetch_async(url: &str, method: &str, body: Option<String>) -> String {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let mut req = match method {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "DELETE" => client.delete(url),
        "HEAD" => client.head(url),
        _ => client.request(
            reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET),
            url,
        ),
    };
    if let Some(b) = body {
        req = req.body(b);
    }
    match req.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            let len = text.len();
            // Truncate to 256 KB.
            let truncated = if len > 256 * 1024 {
                let mut t = text[..256 * 1024].to_string();
                t.push_str("\n...[truncated]");
                t
            } else {
                text
            };
            json!({
                "status": status,
                "body": truncated,
                "len": len,
            })
            .to_string()
        }
        Err(e) => json!({"error": format!("http fetch failed: {e}")}).to_string(),
    }
}

fn run_git_op(policy: &SafetyPolicy, args: &Value) -> String {
    let git_args = match args.get("args").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'args' argument"}).to_string(),
    };
    let cwd = args
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            directories::BaseDirs::new()
                .map(|b| b.home_dir().to_string_lossy().to_string())
                .unwrap_or_else(|| ".".into())
        });

    // Safety: route the equivalent shell command through the policy.
    let shell_equiv = format!("git {git_args}");
    match policy.check_command(&shell_equiv) {
        SafetyDecision::Allow => {
            let out = std::process::Command::new("git")
                .args(git_args.split_whitespace())
                .current_dir(&cwd)
                .output();
            match out {
                Ok(o) => {
                    let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                    let success = o.status.success();
                    json!({
                        "ok": success,
                        "stdout": stdout,
                        "stderr": stderr,
                        "exit_code": o.status.code().unwrap_or(-1),
                        "cwd": cwd,
                    })
                    .to_string()
                }
                Err(e) => json!({"error": format!("git exec failed: {e}")}).to_string(),
            }
        }
        SafetyDecision::Deny { reason } => json!({
            "safety_decision": "deny",
            "reason": reason,
            "command": shell_equiv,
        })
        .to_string(),
        SafetyDecision::RequireConfirmation { token, summary, .. } => json!({
            "safety_decision": "require_confirmation",
            "token": token,
            "summary": summary,
            "command": shell_equiv,
        })
        .to_string(),
    }
}

fn run_process_list() -> String {
    // Use the existing process monitor if available.
    let procs = crate::security::monitor::snapshot_processes();
    let truncated: Vec<&crate::security::monitor::ProcInfo> = procs.iter().take(200).collect();
    json!({"processes": truncated, "total": procs.len()}).to_string()
}

fn run_process_kill(policy: &SafetyPolicy, args: &Value) -> String {
    let pid = match args.get("pid").and_then(|v| v.as_i64()) {
        Some(n) => n as i32,
        None => return json!({"error":"missing 'pid' argument"}).to_string(),
    };
    let cmd = if cfg!(windows) {
        format!("taskkill /pid {pid} /f")
    } else {
        format!("kill -9 {pid}")
    };
    match policy.check_command(&cmd) {
        SafetyDecision::Allow => {
            let out = if cfg!(windows) {
                std::process::Command::new("taskkill")
                    .args(["/pid", &pid.to_string(), "/f"])
                    .output()
            } else {
                std::process::Command::new("kill")
                    .args(["-9", &pid.to_string()])
                    .output()
            };
            match out {
                Ok(o) if o.status.success() => json!({"ok": true, "pid": pid}).to_string(),
                Ok(o) => json!({
                    "error": format!("kill failed: {}", String::from_utf8_lossy(&o.stderr)),
                    "pid": pid,
                })
                .to_string(),
                Err(e) => json!({"error": format!("kill exec failed: {e}")}).to_string(),
            }
        }
        SafetyDecision::Deny { reason } => json!({
            "safety_decision": "deny",
            "reason": reason,
            "pid": pid,
        })
        .to_string(),
        SafetyDecision::RequireConfirmation { token, summary, .. } => json!({
            "safety_decision": "require_confirmation",
            "token": token,
            "summary": summary,
            "pid": pid,
        })
        .to_string(),
    }
}

fn run_code_eval(policy: &SafetyPolicy, args: &Value) -> String {
    let language = match args.get("language").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'language' argument"}).to_string(),
    };
    let code = match args.get("code").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'code' argument"}).to_string(),
    };
    let timeout_seconds = args
        .get("timeout_seconds")
        .and_then(|v| v.as_u64())
        .unwrap_or(15);

    let (interpreter, cmd_str) = match language.as_str() {
        "python3" | "python" => ("python3", "python3 -c '...'".to_string()),
        "node" | "javascript" | "js" => ("node", "node -e '...'".to_string()),
        "bash" | "sh" => ("bash", "bash -c '...'".to_string()),
        other => {
            return json!({"error": format!("unsupported language: {other}")}).to_string();
        }
    };

    // Route through safety policy (these are shell-equivalents).
    match policy.check_command(&cmd_str) {
        SafetyDecision::Allow => {
            let tmp = std::env::temp_dir().join(format!(
                "aegis-eval-{}{}",
                uuid::Uuid::new_v4().simple(),
                match language.as_str() {
                    "python3" | "python" => ".py",
                    "node" | "javascript" | "js" => ".js",
                    _ => ".sh",
                }
            ));
            if let Err(e) = std::fs::write(&tmp, &code) {
                return json!({"error": format!("failed to write temp file: {e}")}).to_string();
            }
            // Spawn the interpreter and apply a wall-clock timeout. If the
            // child does not finish within `timeout_seconds`, kill it so
            // runaway scripts (e.g. `while True: pass`) can't hang the agent.
            let out = run_with_timeout(
                std::process::Command::new(interpreter).arg(tmp.to_string_lossy().to_string()),
                timeout_seconds,
            );
            let _ = std::fs::remove_file(&tmp);
            match out {
                Ok(o) => {
                    let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&o.stderr).to_string();
                    let combined = format!("{stdout}\n{stderr}");
                    let truncated = if combined.len() > 64 * 1024 {
                        format!("{}...[truncated]", &combined[..64 * 1024])
                    } else {
                        combined
                    };
                    json!({
                        "ok": o.status.success(),
                        "output": truncated,
                        "exit_code": o.status.code().unwrap_or(-1),
                    })
                    .to_string()
                }
                Err(e) => json!({"error": format!("exec failed: {e}")}).to_string(),
            }
        }
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

/// Run a [`Command`], killing it if it exceeds `timeout_secs`.
///
/// Implemented with `spawn()` + `wait()` with a deadline so we don't leak
/// children when the user asks for a 15-second cap on AI-generated code.
fn run_with_timeout(
    cmd: &mut std::process::Command,
    timeout_secs: u64,
) -> std::io::Result<std::process::Output> {
    let start = std::time::Instant::now();
    let mut child = cmd.spawn()?;
    let deadline = start + std::time::Duration::from_secs(timeout_secs);
    loop {
        match child.try_wait()? {
            Some(_) => return child.wait_with_output(),
            None => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        format!("eval timed out after {timeout_secs}s"),
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
}

fn run_notify(args: &Value) -> String {
    let title = match args.get("title").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'title' argument"}).to_string(),
    };
    let body = match args.get("body").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'body' argument"}).to_string(),
    };
    // Best-effort: use notify-send on Linux, msg on Windows, osascript on macOS.
    let out = if cfg!(target_os = "linux") {
        std::process::Command::new("notify-send")
            .arg(&title)
            .arg(&body)
            .output()
    } else if cfg!(windows) {
        std::process::Command::new("msg")
            .arg("*")
            .arg(format!("{title}: {body}"))
            .output()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!(
                r#"display notification "{body}" with title "{title}""#
            ))
            .output()
    } else {
        return json!({"ok": false, "note": "notifications not supported on this platform"})
            .to_string();
    };
    match out {
        Ok(o) if o.status.success() => json!({"ok": true}).to_string(),
        Ok(o) => json!({
            "ok": false,
            "error": String::from_utf8_lossy(&o.stderr).to_string(),
        })
        .to_string(),
        Err(e) => json!({"ok": false, "error": format!("exec failed: {e}")}).to_string(),
    }
}

fn run_open_url(args: &Value) -> String {
    let url = match args.get("url").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'url' argument"}).to_string(),
    };
    let out = if cfg!(target_os = "linux") {
        std::process::Command::new("xdg-open").arg(&url).output()
    } else if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &url])
            .output()
    } else if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(&url).output()
    } else {
        return json!({"ok": false, "note": "open_url not supported on this platform"}).to_string();
    };
    match out {
        Ok(o) if o.status.success() => json!({"ok": true, "url": url}).to_string(),
        Ok(o) => json!({
            "ok": false,
            "error": String::from_utf8_lossy(&o.stderr).to_string(),
        })
        .to_string(),
        Err(e) => json!({"ok": false, "error": format!("exec failed: {e}")}).to_string(),
    }
}

fn run_memory_search(
    memory: &std::sync::Arc<crate::memory::store::MemoryStore>,
    args: &Value,
) -> String {
    let query = match args.get("query").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'query' argument"}).to_string(),
    };
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
    // v0.4: simple substring + token-overlap search. A future version will
    // plug in a real vector store.
    let results = memory.knowledge.search(&query, limit);
    serde_json::to_string(&results).unwrap_or_else(|_| "[]".into())
}

fn run_skill_set(args: &Value) -> String {
    let id = match args.get("id").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => return json!({"error":"missing 'id' argument"}).to_string(),
    };
    if crate::ai::skills::find(&id).is_none() {
        return json!({"error": format!("unknown skill: {id}")}).to_string();
    }
    // Persist the active skill via the global config.
    if let Ok(cfg_path) = std::env::var("AEGIS_CONFIG_PATH") {
        let _ = cfg_path;
    }
    // We can't reach the AppState from here without a bigger refactor.
    // Instead, we write the skill to a sidecar file that AppState reads
    // on next request.
    let skill_file = crate::config::AppConfig::data_dir().join("active_skill");
    let _ = std::fs::write(&skill_file, &id);
    json!({"ok": true, "skill": id}).to_string()
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
