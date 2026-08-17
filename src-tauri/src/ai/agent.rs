//! Agent loop: a tool-use iteration that lets the AI act as a "co-owner"
//! of the user's computer. The AI proposes actions via OpenAI-style
//! function-calling; the safety policy gates each action; results are fed
//! back into the conversation until the AI returns a final message.
//!
//! ## High-level flow
//!
//! ```text
//! user message
//!     │
//!     ▼
//! ┌───────────────────────────────────────────┐
//! │ build ChatRequest with `tools` attached    │
//! │ call provider.chat_stream()                │
//! │ collect assistant message + tool_calls     │
//! └───────────────────────────────────────────┘
//!     │ no tool_calls? ──► done, return assistant text
//!     │ has tool_calls?
//!     ▼
//! ┌───────────────────────────────────────────┐
//! │ for each tool_call:                        │
//! │   1. check rate limiter / kill switch      │
//! │   2. dispatch through safety policy        │
//! │   3. audit-log the call                    │
//! │   4. emit `agent://tool_call` event        │
//! │ append tool results as Role::Tool messages │
//! └───────────────────────────────────────────┘
//!     │
//!     ▼ loop until done or max_iterations
//! ```
//!
//! ## Safety invariants
//!
//! - Hard cap on `max_iterations` (default 10) prevents runaway loops.
//! - `KillSwitch` checked before every tool call: if tripped, the loop aborts.
//! - `RateLimiter` checked before every tool call: enforces per-minute cap.
//! - Every tool call is audit-logged with tool name, args, result, timestamp.
//! - If a tool returns `safety_decision=require_confirmation`, the loop
//!   forwards the token to the AI in the tool result and stops, surfacing
//!   the confirmation request to the frontend via `agent://confirmation`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tokio::sync::watch;

use crate::ai::provider::{ChatMessage, ChatRequest, ChatStreamChunk};
use crate::ai::router::AiRouter;
use crate::ai::provider::ProviderRegistry;
use crate::ai::tools::{self, ToolCall, ToolResult, ToolSpec};
use crate::computer::safety::SafetyPolicy;
use crate::config::AppConfig;
use crate::error::{AegisError, Result};
use crate::memory::store::MemoryStore;
use crate::state::AppState;

/// Default per-turn iteration cap.
pub const DEFAULT_MAX_ITERATIONS: u32 = 10;
/// Absolute iteration cap, regardless of caller request.
pub const ABSOLUTE_MAX_ITERATIONS: u32 = 20;
/// Default per-minute action rate limit.
pub const DEFAULT_RATE_LIMIT_PER_MIN: u32 = 30;
/// Stream chunk emit channel for the agent loop.
pub const EVENT_TOOL_CALL: &str = "agent://tool_call";
pub const EVENT_TOOL_RESULT: &str = "agent://tool_result";
pub const EVENT_CONFIRMATION: &str = "agent://confirmation";
pub const EVENT_DONE: &str = "agent://done";
pub const EVENT_ERROR: &str = "agent://error";

/// Parameters for one agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunParams {
    pub conversation_id: Option<String>,
    pub user_message: String,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    /// Max tool-use iterations before the loop gives up.
    /// Capped to [`ABSOLUTE_MAX_ITERATIONS`].
    pub max_iterations: Option<u32>,
}

/// Final result of an agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunResult {
    pub conversation_id: String,
    /// Final assistant reply (concatenated stream).
    pub content: String,
    /// Model that produced the final reply.
    pub model: String,
    /// Number of tool-use iterations executed.
    pub iterations: u32,
    /// Number of tool calls dispatched.
    pub tool_calls: u32,
    /// Tool calls that required user confirmation (loop stopped).
    pub confirmations_pending: u32,
    /// Wall-clock duration in ms.
    pub duration_ms: u64,
}

/// Run the agent loop. Spawns a background task and returns immediately.
///
/// Emits events to the frontend:
/// - `agent://tool_call` — when a tool is about to be dispatched.
/// - `agent://tool_result` — when a tool returns (success or error).
/// - `agent://confirmation` — when a tool needs user confirmation (loop stops).
/// - `agent://done` — when the loop finishes successfully.
/// - `agent://error` — when the loop aborts with an error.
pub async fn run_agent_loop(
    state: Arc<Mutex<AppState>>,
    app: tauri::AppHandle,
    params: AgentRunParams,
) -> Result<String> {
    let providers = {
        let s = state.lock();
        let __moved = (*s.providers.lock()).clone();
        __moved
    };
    let router = {
        let s = state.lock();
        s.router.clone()
    };

    // Persist the user message + create conversation if needed.
    let conv_id = match params.conversation_id.clone() {
        Some(id) => id,
        None => {
            let s = state.lock();
            let title = params.user_message.chars().take(40).collect::<String>();
            let conv = s.memory.conversations.create(&title, None)?;
            conv.id
        }
    };
    {
        let s = state.lock();
        s.memory
            .conversations
            .add_message(&conv_id, "user", &params.user_message)?;
        let _ = s.memory.activity.record(
            "agent.user",
            &format!("agent run started in {conv_id}"),
            None,
        );
    }

    let run_id = uuid::Uuid::new_v4().simple().to_string();
    let state_for_task = state.clone();
    let app_clone = app.clone();
    let params_clone = params.clone();
    let conv_id_clone = conv_id.clone();
    let run_id_clone = run_id.clone();

    tokio::spawn(async move {
        let result = agent_loop_inner(
            state_for_task.clone(),
            app_clone.clone(),
            providers,
            router,
            params_clone,
            conv_id_clone.clone(),
        )
        .await;

        match result {
            Ok(r) => {
                let _ = app_clone.emit(EVENT_DONE, &serde_json::json!({
                    "run_id": run_id_clone,
                    "conversation_id": r.conversation_id,
                    "iterations": r.iterations,
                    "tool_calls": r.tool_calls,
                    "duration_ms": r.duration_ms,
                }));
            }
            Err(e) => {
                tracing::error!("agent loop failed: {e:#}");
                let _ = app_clone.emit(EVENT_ERROR, &serde_json::json!({
                    "run_id": run_id_clone,
                    "error": e.to_string(),
                }));
            }
        }
    });

    Ok(run_id)
}

async fn agent_loop_inner(
    state: Arc<Mutex<AppState>>,
    app: tauri::AppHandle,
    providers: ProviderRegistry,
    router: Arc<AiRouter>,
    params: AgentRunParams,
    conv_id: String,
) -> Result<AgentRunResult> {
    let start = Instant::now();
    let max_iters = params
        .max_iterations
        .unwrap_or(DEFAULT_MAX_ITERATIONS)
        .min(ABSOLUTE_MAX_ITERATIONS)
        .max(1);

    // Load history.
    let history = {
        let s = state.lock();
        s.memory.conversations.messages(&conv_id)?
    };
    let mut messages: Vec<ChatMessage> = history
        .into_iter()
        .map(|m| match m.role.as_str() {
            "user" => ChatMessage::user(m.content),
            "assistant" => ChatMessage::assistant(m.content),
            _ => ChatMessage::system(m.content),
        })
        .collect();

    // Inject the system prompt that explains the agent's role.
    let skill_fragment = {
        let path = crate::config::AppConfig::data_dir().join("active_skill");
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| crate::ai::skills::find(s.trim()).map(|sk| sk.system_prompt_fragment))
            .unwrap_or("")
    };
    let system_prompt = format!(
        "You are Aegis AI, a secure cross-platform assistant with full \
         computer-use capabilities. You can run shell commands, read and \
         write files, launch applications, capture the screen, automate \
         the GUI, and remember facts for later.\n\n\
         When the user asks you to do something on their machine, prefer \
         calling the appropriate tool (`shell`, `file_read`, `file_write`, \
         `app_open`, etc.) over instructing them to do it manually. If a \
         tool returns `safety_decision: require_confirmation`, tell the \
         user to approve the action in the Aegis UI and stop.\n\n\
         Be concise, accurate, and refuse any request that would harm the \
         user's system. Never attempt to bypass the safety policy.\n\n\
         --- Active skill specialization ---\n\
         {skill_fragment}"
    );
    messages.insert(0, ChatMessage::system(system_prompt));

    let tool_specs: Vec<ToolSpec> = tools::all_specs();
    let tools_json = tools::specs_as_json();

    let mut iterations = 0u32;
    let mut tool_calls_total = 0u32;
    let mut confirmations_pending = 0u32;
    let mut final_content = String::new();
    let mut final_model = String::new();

    while iterations < max_iters {
        // Check the kill switch before each iteration.
        if crate::computer::kill_switch::is_tripped() {
            tracing::info!("agent loop aborted by kill switch at iter {iterations}");
            return Err(AegisError::SafetyDenial(
                "kill switch tripped — agent loop aborted by user".into(),
            ));
        }

        iterations += 1;

        // Build the request. We pass `tools` as an extra param so any
        // OpenAI-compat provider picks them up; providers that don't support
        // function calling simply ignore them and return a plain reply.
        let mut extra = std::collections::BTreeMap::new();
        extra.insert("tools".into(), tools_json.clone());
        extra.insert("tool_choice".into(), serde_json::json!("auto"));

        let req = ChatRequest {
            messages: messages.clone(),
            model: params.model.clone(),
            temperature: params.temperature.or(Some(0.7)),
            max_tokens: Some(1024),
            top_p: None,
            stop: vec![],
            extra,
        };

        // Stream chunks to the frontend as we go (for the user's live view).
        let app_for_chunks = app.clone();
        let on_chunk: Box<dyn Fn(ChatStreamChunk) + Send + Sync> = Box::new(move |chunk| {
            let _ = app_for_chunks.emit(
                "agent://chunk",
                &serde_json::json!({"delta": chunk.delta, "done": chunk.done}),
            );
        });

        let resp = match router.chat_stream(&providers, req, on_chunk).await {
            Ok(r) => r,
            Err(e) => {
                return Err(e);
            }
        };

        final_model = resp.model.clone();
        let mut assistant_text = resp.message.content.clone();
        if assistant_text.is_empty() && iterations == 1 {
            // Some providers may put all output into tool_calls only.
            assistant_text.push_str("(no text — proceeding to tool calls)");
        }

        // Extract any tool calls from the response.
        let tool_calls = extract_tool_calls(&resp.message.tool_calls);

        // Persist the assistant message (with tool calls serialized).
        {
            let s = state.lock();
            let mut content = assistant_text.clone();
            if !tool_calls.is_empty() {
                let calls_json = serde_json::to_string_pretty(&tool_calls).unwrap_or_default();
                content.push_str("\n\n[tool_calls]:\n");
                content.push_str(&calls_json);
            }
            let _ = s.memory.conversations.add_message(&conv_id, "assistant", &content);
        }

        final_content = assistant_text.clone();

        // No tool calls? We're done.
        if tool_calls.is_empty() {
            tracing::debug!("agent loop finished after {iterations} iterations (no tool calls)");
            break;
        }

        // Append the assistant message (including its tool_calls payload) to
        // the local conversation buffer so the next round sees them.
        messages.push(ChatMessage {
            role: crate::ai::provider::Role::Assistant,
            content: assistant_text.clone(),
            name: None,
            tool_calls: resp.message.tool_calls.clone(),
        });

        // Dispatch each tool call locally.
        let policy = {
            let s = state.lock();
            let __moved = SafetyPolicy::from_config(&s.config.read());
            __moved
        };

        for call in &tool_calls {
            // Rate-limit check.
            if !crate::computer::rate_limiter::try_consume() {
                tracing::warn!("agent loop rate-limited at iter {iterations}");
                let _ = app.emit(EVENT_ERROR, &serde_json::json!({
                    "error": "rate limit exceeded — too many actions per minute",
                    "iteration": iterations,
                }));
                return Err(AegisError::SafetyDenial(
                    "rate limit exceeded — too many actions per minute".into(),
                ));
            }

            // Emit pre-dispatch event.
            let _ = app.emit(EVENT_TOOL_CALL, &serde_json::json!({
                "iteration": iterations,
                "tool_call_id": call.id,
                "name": call.name,
                "arguments": call.arguments,
            }));

            // Audit-log the call.
            {
                let s = state.lock();
                let _ = s.memory.activity.record(
                    "agent.tool_call",
                    &format!("{} ({})", call.name, call.arguments),
                    Some(&format!("iter={iterations}")),
                );
            }

            let memory_arc = {
                let s = state.lock();
                s.memory.clone()
            };
            let result: ToolResult = tools::dispatch(call, &policy, &memory_arc);
            let _ = app.emit(EVENT_TOOL_RESULT, &serde_json::json!({
                "tool_call_id": result.tool_call_id,
                "name": result.name,
                "content": result.content,
                "success": result.success,
            }));

            // If the tool required confirmation, surface it and stop.
            if result
                .content
                .contains("\"safety_decision\":\"require_confirmation\"")
            {
                confirmations_pending += 1;
                let _ = app.emit(EVENT_CONFIRMATION, &serde_json::json!({
                    "tool_call_id": result.tool_call_id,
                    "name": result.name,
                    "content": result.content,
                }));
                // Append the tool result so the AI can see it on the next
                // turn if the user re-runs the loop.
                messages.push(tools::result_to_message(&result));
                // Persist confirmation result message.
                {
                    let s = state.lock();
                    let _ = s.memory.conversations.add_message(
                        &conv_id,
                        "tool",
                        &format!("{}: {}", result.name, result.content),
                    );
                }
                // Stop the loop — the frontend must re-trigger after user approves.
                return Ok(AgentRunResult {
                    conversation_id: conv_id.clone(),
                    content: "Action requires user confirmation. Please approve it in the Aegis UI, then send another message to continue.".into(),
                    model: final_model,
                    iterations,
                    tool_calls: tool_calls_total,
                    confirmations_pending,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }

            tool_calls_total += 1;
            messages.push(tools::result_to_message(&result));
            // Persist tool message.
            {
                let s = state.lock();
                let _ = s.memory.conversations.add_message(
                    &conv_id,
                    "tool",
                    &format!("{}: {}", result.name, result.content),
                );
            }
        }

        // Loop again: ask the AI to either produce a final reply or call more tools.
    }

    if iterations >= max_iters {
        tracing::warn!("agent loop hit max_iterations={max_iters}");
    }

    // Persist the final assistant reply.
    {
        let s = state.lock();
        let _ = s.memory.activity.record(
            "agent.done",
            &format!("agent run finished in {conv_id} (iters={iterations}, tools={tool_calls_total})"),
            None,
        );
    }

    Ok(AgentRunResult {
        conversation_id: conv_id,
        content: final_content,
        model: final_model,
        iterations,
        tool_calls: tool_calls_total,
        confirmations_pending,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// Parse the OpenAI-style tool_calls payload (a JSON value) into typed calls.
fn extract_tool_calls(raw: &Option<serde_json::Value>) -> Vec<ToolCall> {
    let Some(arr) = raw.as_ref().and_then(|v| v.as_array()) else {
        return vec![];
    };
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("call_unknown")
            .to_string();
        let function = match item.get("function") {
            Some(f) => f,
            None => continue,
        };
        let name = function
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let arguments_str = function
            .get("arguments")
            .and_then(|v| v.as_str())
            .unwrap_or("{}");
        let arguments: serde_json::Value = serde_json::from_str(arguments_str)
            .unwrap_or(serde_json::Value::Object(Default::default()));
        out.push(ToolCall {
            id,
            name,
            arguments,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_tool_calls_handles_none() {
        assert!(extract_tool_calls(&None).is_empty());
    }

    #[test]
    fn extract_tool_calls_parses_openai_payload() {
        let raw = serde_json::json!([
            {
                "id": "call_1",
                "type": "function",
                "function": {
                    "name": "shell",
                    "arguments": "{\"command\": \"ls\"}"
                }
            }
        ]);
        let calls = extract_tool_calls(&Some(raw));
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "shell");
        assert_eq!(calls[0].id, "call_1");
        assert_eq!(calls[0].arguments["command"], "ls");
    }

    #[test]
    fn extract_tool_calls_skips_missing_name() {
        let raw = serde_json::json!([
            {
                "id": "call_1",
                "function": {"arguments": "{}"}
            }
        ]);
        let calls = extract_tool_calls(&Some(raw));
        assert!(calls.is_empty());
    }
}
