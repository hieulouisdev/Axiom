//! Tauri command handlers: the IPC bridge between the Rust backend and the
//! React frontend.
//!
//! Every command is registered in `lib.rs::run` via `tauri::generate_handler!`.

use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::Emitter;
use tauri::State;

use crate::{
    SharedState,
    ai::provider::{ChatMessage, ChatRequest, ChatStreamChunk},
    computer::{
        apps::{AppDescriptor, list_apps, open_app},
        automation::{AutoAction, auto_perform},
        clipboard::{
            ClipboardContent, clipboard_read, clipboard_watch_start, clipboard_watch_stop,
            clipboard_write,
        },
        commands::{ExecResult, exec_command},
        files::{FileReadResult, file_read, file_write},
        safety::SafetyPolicy,
        screenshot,
    },
    config::{OperatingMode, ProviderCredentials},
    error::{AegisError, Result},
    i18n,
    memory::{Conversation, Message},
    modes::Mode,
    security::{
        self, DefenseEvent, integrity::IntegrityEvent, monitor::Threat, network::NetworkAnomaly,
        quarantine::QuarantineEntry, sandbox::SandboxPolicy, scanner::ScanResult,
    },
    state::AppState,
};

// ===========================================================================
// AI
// ===========================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatParams {
    pub conversation_id: Option<String>,
    pub user_message: String,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[tauri::command]
pub async fn ai_chat(
    state: State<'_, Arc<Mutex<AppState>>>,
    params: ChatParams,
) -> Result<ChatResponseDto> {
    let providers = {
        let s = state.lock();

        (*s.providers.lock()).clone()
    };
    let router = {
        let s = state.lock();
        s.router.clone()
    };

    let conv_id = match params.conversation_id.clone() {
        Some(id) => id,
        None => {
            let s = state.lock();
            let title = params.user_message.chars().take(40).collect::<String>();
            let conv = s.memory.conversations.create(&title, None)?;
            conv.id
        }
    };

    // Persist the user message.
    {
        let s = state.lock();
        s.memory
            .conversations
            .add_message(&conv_id, "user", &params.user_message)?;
        let _ = s
            .memory
            .activity
            .record("chat.user", &format!("user message in {conv_id}"), None);
    }

    // Load conversation history.
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

    // Prepend a system prompt.
    messages.insert(
        0,
        ChatMessage::system(
            "You are Aegis AI, a secure cross-platform assistant. \
             Be concise, accurate, and refuse any request that would harm the user's system. \
             When the user asks you to perform an action on their computer, request the action \
             through the computer_use tool — never instruct the user to type shell commands manually."
        ),
    );

    // v0.5: Retrieval-augmented generation — inject up to 5 stored facts
    // whose embeddings are most similar to the user's latest message.
    // No-op if the knowledge base is empty.
    {
        let s = state.lock();
        let _ = crate::memory::rag::inject_default(
            &mut messages,
            &params.user_message,
            &s.memory.embeddings,
        );

        // v0.6: Auto-extract entities from the latest user message and
        // persist any new facts to the knowledge base. Best-effort.
        let texts = vec![params.user_message.clone()];
        match crate::memory::entities::extract_and_store(&s.memory, &texts) {
            Ok(n) if n > 0 => tracing::info!("extracted {n} new entities from user message"),
            Ok(_) => {}
            Err(e) => tracing::warn!("entity extraction failed: {e}"),
        }
    }

    let req = ChatRequest {
        messages,
        model: params.model,
        temperature: params.temperature,
        max_tokens: params.max_tokens,
        top_p: None,
        stop: vec![],
        extra: Default::default(),
    };

    let resp = router.chat(&providers, req).await?;

    // Persist the assistant reply.
    {
        let s = state.lock();
        s.memory
            .conversations
            .add_message(&conv_id, "assistant", &resp.message.content)?;
        let _ = s.memory.activity.record(
            "chat.assistant",
            &format!(
                "assistant replied in {conv_id} ({} chars)",
                resp.message.content.len()
            ),
            None,
        );
    }

    Ok(ChatResponseDto {
        conversation_id: conv_id,
        content: resp.message.content,
        model: resp.model,
        usage: resp.usage.map(|u| UsageDto {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }),
    })
}

#[derive(Debug, Serialize)]
pub struct ChatResponseDto {
    pub conversation_id: String,
    pub content: String,
    pub model: String,
    pub usage: Option<UsageDto>,
}

#[derive(Debug, Serialize)]
pub struct UsageDto {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ===========================================================================
// Streaming Chat (Phase 2)
// ===========================================================================

#[tauri::command]
pub async fn ai_chat_stream(
    state: State<'_, Arc<Mutex<AppState>>>,
    app: tauri::AppHandle,
    params: ChatParams,
) -> Result<ChatStreamStartDto> {
    let providers = {
        let s = state.lock();

        (*s.providers.lock()).clone()
    };
    let router = {
        let s = state.lock();
        s.router.clone()
    };

    let conv_id = match params.conversation_id.clone() {
        Some(id) => id,
        None => {
            let s = state.lock();
            let title = params.user_message.chars().take(40).collect::<String>();
            let conv = s.memory.conversations.create(&title, None)?;
            conv.id
        }
    };

    // Persist user message
    {
        let s = state.lock();
        s.memory
            .conversations
            .add_message(&conv_id, "user", &params.user_message)?;
        let _ = s.memory.activity.record(
            "chat.user",
            &format!("streaming user message in {conv_id}"),
            None,
        );
    }

    // Load history
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
    messages.insert(
        0,
        ChatMessage::system(
            "You are Aegis AI, a secure cross-platform assistant. \
         Be concise, accurate, and refuse any request that would harm the user's system.",
        ),
    );

    // v0.5: RAG — inject relevant stored facts into the system message.
    {
        let s = state.lock();
        let _ = crate::memory::rag::inject_default(
            &mut messages,
            &params.user_message,
            &s.memory.embeddings,
        );

        // v0.6: Auto-extract entities from the latest user message.
        let texts = vec![params.user_message.clone()];
        match crate::memory::entities::extract_and_store(&s.memory, &texts) {
            Ok(n) if n > 0 => tracing::info!("extracted {n} new entities from user message"),
            Ok(_) => {}
            Err(e) => tracing::warn!("entity extraction failed: {e}"),
        }
    }

    // Generate stream ID and create cancel token
    let stream_id = uuid::Uuid::new_v4().simple().to_string();
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    {
        let s = state.lock();
        s.cancel_tokens.lock().insert(stream_id.clone(), cancel_tx);
    }

    let stream_id_clone = stream_id.clone();
    let conv_id_clone = conv_id.clone();
    let state_clone = state.inner().clone();

    // Spawn the streaming task
    tokio::spawn(async move {
        let app_for_chunk = app.clone();
        // Clone stream_id once per closure scope so we can use it in both the
        // chunk callback and the error handler without move conflicts.
        let stream_id_for_chunk = stream_id_clone.clone();
        let stream_id_for_err = stream_id_clone.clone();
        let on_chunk: Box<dyn Fn(ChatStreamChunk) + Send + Sync> = Box::new(move |chunk| {
            let _ = app_for_chunk.emit(
                "chat://chunk",
                &serde_json::json!({
                    "stream_id": stream_id_for_chunk,
                    "delta": chunk.delta,
                    "done": chunk.done,
                }),
            );
        });

        let req = ChatRequest {
            messages,
            model: params.model,
            temperature: params.temperature,
            max_tokens: params.max_tokens,
            top_p: None,
            stop: vec![],
            extra: Default::default(),
        };

        // Check for cancellation
        let mut cancelled = false;
        tokio::select! {
            result = router.chat_stream(&providers, req, on_chunk) => {
                match result {
                    Ok(resp) => {
                        // Persist the full response
                        let s = state_clone.lock();
                        let _ = s.memory.conversations.add_message(&conv_id_clone, "assistant", &resp.message.content);
                        let _ = s.memory.activity.record("chat.assistant", &format!("streaming reply in {conv_id_clone} ({} chars)", resp.message.content.len()), None);
                    }
                    Err(e) => {
                        let _ = app.emit("chat://error", &serde_json::json!({
                            "stream_id": stream_id_for_err,
                            "error": e.to_string(),
                        }));
                    }
                }
            }
            _ = async {
                let mut rx = cancel_rx;
                while !*rx.borrow() {
                    if rx.changed().await.is_err() {
                        break;
                    }
                }
            } => {
                cancelled = true;
                let _ = app.emit("chat://cancelled", &serde_json::json!({
                    "stream_id": stream_id_clone,
                }));
            }
        }

        // Clean up cancel token
        let s = state_clone.lock();
        s.cancel_tokens.lock().remove(&stream_id_clone);

        if !cancelled {
            let _ = app.emit(
                "chat://done",
                &serde_json::json!({
                    "stream_id": stream_id_clone,
                }),
            );
        }
    });

    Ok(ChatStreamStartDto {
        stream_id,
        conversation_id: conv_id,
    })
}

#[derive(Debug, Serialize)]
pub struct ChatStreamStartDto {
    pub stream_id: String,
    pub conversation_id: String,
}

#[tauri::command]
pub fn ai_chat_cancel(state: State<'_, Arc<Mutex<AppState>>>, stream_id: String) -> Result<()> {
    let s = state.lock();
    let mut tokens = s.cancel_tokens.lock();
    match tokens.remove(&stream_id) {
        Some(tx) => {
            let _ = tx.send(true);
            tracing::info!("cancelled stream: {}", stream_id);
            Ok(())
        }
        _ => Err(AegisError::Internal(format!(
            "no active stream with id {stream_id}"
        ))),
    }
}

// ===========================================================================
// Provider management
// ===========================================================================

#[tauri::command]
pub fn ai_list_providers(state: State<'_, Arc<Mutex<AppState>>>) -> Vec<ProviderDto> {
    let s = state.lock();
    let registry = s.providers.lock();
    let cfg = s.config.read();
    registry
        .list()
        .into_iter()
        .map(|d| {
            // Pre-compute the active flag before moving `d.id` into the struct.
            let is_active = cfg.active_provider.as_deref() == Some(&d.id);
            let creds = cfg.providers.credentials.get(&d.id);
            // Pre-compute flags that depend on `d`/`creds` so we don't borrow
            // moved values later.
            let requires_api_key = d.requires_api_key;
            let enabled = creds.map(|c| c.enabled).unwrap_or(false);
            let configured = creds
                .map(|c| c.api_key.is_some() || !requires_api_key || c.base_url.is_some())
                .unwrap_or(false);
            ProviderDto {
                id: d.id,
                name: d.name,
                description: d.description,
                homepage: d.homepage,
                category: format!("{:?}", d.category).to_lowercase(),
                requires_api_key: d.requires_api_key,
                local: d.local,
                default_base_url: d.default_base_url,
                default_model: d.default_model,
                known_models: d.known_models,
                implemented: d.implemented,
                enabled,
                is_active,
                configured,
            }
        })
        .collect()
}

#[derive(Debug, Serialize)]
pub struct ProviderDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub homepage: String,
    pub category: String,
    pub requires_api_key: bool,
    pub local: bool,
    pub default_base_url: Option<String>,
    pub default_model: String,
    pub known_models: Vec<String>,
    pub implemented: bool,
    pub enabled: bool,
    pub is_active: bool,
    pub configured: bool,
}

#[tauri::command]
pub fn ai_set_active_provider(
    state: State<'_, Arc<Mutex<AppState>>>,
    provider_id: Option<String>,
) -> Result<()> {
    let router = {
        let s = state.lock();
        s.router.clone()
    };
    router.set_active(provider_id)
}

#[derive(Debug, Deserialize)]
pub struct ProviderConfigDto {
    pub provider_id: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub enabled: bool,
}

#[tauri::command]
pub fn ai_configure_provider(
    state: State<'_, Arc<Mutex<AppState>>>,
    cfg: ProviderConfigDto,
) -> Result<()> {
    // Try to store API key in OS keychain
    if let Some(ref api_key) = cfg.api_key {
        match keyring::Entry::new("aegis-ai", &cfg.provider_id) {
            Ok(entry) => match entry.set_password(api_key) {
                Err(e) => {
                    tracing::warn!(
                        "keyring store failed for {}, falling back to config: {e}",
                        cfg.provider_id
                    );
                }
                _ => {
                    tracing::debug!("stored api_key in keyring for {}", cfg.provider_id);
                }
            },
            Err(e) => {
                tracing::warn!("keyring entry creation failed for {}: {e}", cfg.provider_id);
            }
        }
    }

    {
        let s = state.lock();
        let mut config = s.config.write();
        config.providers.credentials.insert(
            cfg.provider_id.clone(),
            ProviderCredentials {
                // Store a placeholder if keyring succeeded; actual key in keyring
                api_key: cfg.api_key.clone(),
                base_url: cfg.base_url,
                model: cfg.model,
                enabled: cfg.enabled,
                extra: Default::default(),
            },
        );
        config.save()?;
    }
    let router = {
        let s = state.lock();
        s.router.clone()
    };
    router.refresh();
    Ok(())
}

#[tauri::command]
pub async fn ai_test_provider(
    state: State<'_, Arc<Mutex<AppState>>>,
    provider_id: String,
) -> Result<()> {
    let providers = {
        let s = state.lock();

        (*s.providers.lock()).clone()
    };
    let provider = providers.get(&provider_id).ok_or_else(|| {
        AegisError::AiNotConfigured(format!("provider '{provider_id}' not registered"))
    })?;

    // Inject current credentials.
    let creds = {
        let s = state.lock();
        let cfg = s.config.read();
        cfg.providers.credentials.get(&provider_id).cloned()
    };
    if let Some(c) = creds {
        // Try keyring first, fall back to config
        let api_key = match keyring::Entry::new("aegis-ai", &provider_id) {
            Ok(entry) => entry.get_password().ok().or(c.api_key),
            Err(_) => c.api_key,
        };
        let creds = crate::ai::provider::ProviderCreds {
            api_key,
            base_url: c.base_url,
            model: c.model,
            extra: c.extra,
        };
        provider.set_creds(creds);
    }
    provider.ping().await
}

// ===========================================================================
// Computer use
// ===========================================================================

#[derive(Debug, Deserialize)]
pub struct ExecCommandParams {
    pub command: String,
    pub authorized: bool,
}

#[tauri::command]
pub fn computer_exec_command(
    state: State<'_, Arc<Mutex<AppState>>>,
    params: ExecCommandParams,
) -> Result<ExecResult> {
    let policy = {
        let s = state.lock();

        SafetyPolicy::from_config(&s.config.read())
    };
    if params.authorized {
        let r = crate::computer::commands::exec_command_authorized(&params.command)?;
        let s = state.lock();
        let _ = s.memory.activity.record(
            "computer.exec",
            &format!("exec: {}", params.command),
            Some("authorized"),
        );
        return Ok(r);
    }
    let r = exec_command(&policy, &params.command)?;
    let s = state.lock();
    let _ = s
        .memory
        .activity
        .record("computer.exec", &format!("exec: {}", params.command), None);
    Ok(r)
}

#[tauri::command]
pub fn computer_open_app(
    state: State<'_, Arc<Mutex<AppState>>>,
    name: String,
    authorized: bool,
) -> Result<()> {
    let policy = {
        let s = state.lock();

        SafetyPolicy::from_config(&s.config.read())
    };
    if authorized {
        return crate::computer::apps::open_app_authorized(&name);
    }
    open_app(&policy, &name)
}

#[tauri::command]
pub fn computer_list_apps() -> Vec<AppDescriptor> {
    list_apps()
}

#[tauri::command]
pub fn computer_file_read(path: String) -> Result<FileReadResult> {
    file_read(&path)
}

#[derive(Debug, Deserialize)]
pub struct FileWriteParams {
    pub path: String,
    pub content: String,
    pub authorized: bool,
}

#[tauri::command]
pub fn computer_file_write(
    state: State<'_, Arc<Mutex<AppState>>>,
    params: FileWriteParams,
) -> Result<()> {
    let policy = {
        let s = state.lock();

        SafetyPolicy::from_config(&s.config.read())
    };
    if params.authorized {
        return crate::computer::files::file_write_authorized(&params.path, &params.content);
    }
    file_write(&policy, &params.path, &params.content)
}

#[tauri::command]
pub fn computer_screenshot() -> Result<crate::computer::screen::Screenshot> {
    screenshot()
}

#[tauri::command]
pub fn computer_automate(actions: Vec<AutoAction>) -> Result<()> {
    auto_perform(actions)
}

// ===========================================================================
// Confirmation tokens (Phase 2)
// ===========================================================================

/// A pending action awaiting user confirmation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAction {
    pub token: String,
    pub action: String,
    pub summary: String,
    pub created_at_ms: u64,
}

const ACTION_TOKEN_TTL: Duration = Duration::from_secs(60);

#[tauri::command]
pub fn computer_request_action(
    state: State<'_, Arc<Mutex<AppState>>>,
    action: String,
    summary: String,
) -> Result<String> {
    let token = uuid::Uuid::new_v4().simple().to_string();
    let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000;

    let pending = PendingAction {
        token: token.clone(),
        action: action.clone(),
        summary: summary.clone(),
        created_at_ms: now_ms,
    };

    {
        let s = state.lock();
        s.pending_actions.lock().insert(token.clone(), pending);
    }

    tracing::info!("action token created: {} for '{}'", token, summary);
    Ok(token)
}

#[derive(Debug, Deserialize)]
pub struct ConfirmActionParams {
    pub token: String,
    pub action: String,
}

#[tauri::command]
pub fn computer_confirm_action(
    state: State<'_, Arc<Mutex<AppState>>>,
    params: ConfirmActionParams,
) -> Result<()> {
    // v0.3 fix: hold the AppState lock for the duration of the function so the
    // inner `pending_actions` MutexGuard has a stable lifetime.
    let s = state.lock();
    let mut pending_map = s.pending_actions.lock();

    let pending = pending_map.remove(&params.token).ok_or_else(|| {
        AegisError::SafetyDenial(format!("invalid or expired action token: {}", params.token))
    })?;

    // Check expiry (60 seconds)
    let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000;
    let elapsed = now_ms.saturating_sub(pending.created_at_ms);
    if elapsed > ACTION_TOKEN_TTL.as_millis() as u64 {
        return Err(AegisError::SafetyDenial(format!(
            "action token expired ({}ms old)",
            elapsed
        )));
    }

    // Verify action matches
    if pending.action != params.action {
        return Err(AegisError::SafetyDenial(
            "action does not match the confirmed token".into(),
        ));
    }

    tracing::info!("action confirmed: {} -> '{}'", params.token, pending.action);
    Ok(())
}

// ===========================================================================
// Clipboard commands (Phase 2)
// ===========================================================================

#[tauri::command]
pub fn clipboard_read_cmd() -> Result<ClipboardContent> {
    clipboard_read()
}

#[tauri::command]
pub fn clipboard_write_cmd(text: String) -> Result<()> {
    clipboard_write(&text)
}

#[tauri::command]
pub fn clipboard_watch_start_cmd() -> Result<()> {
    clipboard_watch_start()
}

#[tauri::command]
pub fn clipboard_watch_stop_cmd() -> Result<()> {
    clipboard_watch_stop()
}

// ===========================================================================
// Memory
// ===========================================================================

#[tauri::command]
pub fn memory_list_conversations(
    state: State<'_, Arc<Mutex<AppState>>>,
    limit: Option<u32>,
) -> Result<Vec<Conversation>> {
    let s = state.lock();
    s.memory.conversations.list(limit.unwrap_or(50))
}

#[tauri::command]
pub fn memory_get_conversation(
    state: State<'_, Arc<Mutex<AppState>>>,
    conversation_id: String,
) -> Result<Vec<Message>> {
    let s = state.lock();
    s.memory.conversations.messages(&conversation_id)
}

#[tauri::command]
pub fn memory_clear_all(state: State<'_, Arc<Mutex<AppState>>>) -> Result<()> {
    let s = state.lock();
    s.memory.conversations.clear_all()?;
    s.memory.knowledge.clear_all()?;
    Ok(())
}

#[tauri::command]
pub fn memory_search(
    state: State<'_, Arc<Mutex<AppState>>>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<Message>> {
    let s = state.lock();
    s.memory.conversations.search(&query, limit.unwrap_or(50))
}

#[derive(Debug, Serialize)]
pub struct MemoryStatsDto {
    pub conversations: u64,
    pub messages: u64,
    pub activities: u64,
    pub knowledge: u64,
}

#[tauri::command]
pub fn memory_stats(state: State<'_, Arc<Mutex<AppState>>>) -> Result<MemoryStatsDto> {
    let s = state.lock();
    let conn = s.memory.shared_conn();
    let conn = conn.lock();
    let conv: i64 = conn.query_row("SELECT COUNT(*) FROM conversations", [], |r| r.get(0))?;
    let msgs: i64 = conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?;
    let acts: i64 = conn.query_row("SELECT COUNT(*) FROM activities", [], |r| r.get(0))?;
    let knwn: i64 = conn.query_row("SELECT COUNT(*) FROM knowledge", [], |r| r.get(0))?;
    Ok(MemoryStatsDto {
        conversations: conv as u64,
        messages: msgs as u64,
        activities: acts as u64,
        knowledge: knwn as u64,
    })
}

/// Summarize a conversation using the active AI provider.
#[tauri::command]
pub async fn memory_summarize(
    state: State<'_, Arc<Mutex<AppState>>>,
    conversation_id: String,
) -> Result<String> {
    let providers = {
        let s = state.lock();

        (*s.providers.lock()).clone()
    };
    let router = {
        let s = state.lock();
        s.router.clone()
    };

    // Get conversation messages
    let history = {
        let s = state.lock();
        s.memory.conversations.messages(&conversation_id)?
    };

    if history.is_empty() {
        return Err(AegisError::Memory("conversation is empty".into()));
    }

    // Build a summary request
    let conversation_text: String = history
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    let messages = vec![
        ChatMessage::system(
            "You are a summarization assistant. Produce a concise summary of the conversation \
             that captures the key topics, decisions, and outcomes. Keep it under 200 words.",
        ),
        ChatMessage::user(&conversation_text),
    ];

    let req = ChatRequest {
        messages,
        model: None,
        temperature: Some(0.3),
        max_tokens: Some(300),
        top_p: None,
        stop: vec![],
        extra: Default::default(),
    };

    let resp = router.chat(&providers, req).await?;
    let summary = resp.message.content.clone();

    // Store the summary as a system message in the conversation
    {
        let s = state.lock();
        let _ = s.memory.activity.record(
            "memory.summarize",
            &format!("summarized conversation {conversation_id}"),
            None,
        );
    }

    Ok(summary)
}

// ===========================================================================
// Security
// ===========================================================================

#[derive(Debug, Serialize)]
pub struct SecurityStatusDto {
    pub auto_defense: bool,
    pub monitor: bool,
    pub scanner_enabled: bool,
    pub recent_threats: Vec<Threat>,
    pub recent_events: Vec<DefenseEvent>,
    pub network_anomalies: Vec<NetworkAnomaly>,
}

#[tauri::command]
pub fn security_status(state: State<'_, Arc<Mutex<AppState>>>) -> SecurityStatusDto {
    let s = state.lock();
    let cfg = s.config.read();
    let anomalies = crate::security::network::detect_anomalies();
    SecurityStatusDto {
        auto_defense: cfg.security.auto_defense,
        monitor: cfg.security.monitor,
        scanner_enabled: cfg.security.scanner_enabled,
        recent_threats: crate::security::monitor::recent_threats(),
        recent_events: crate::security::defender::recent_events(),
        network_anomalies: anomalies,
    }
}

#[tauri::command]
pub fn security_scan(path: String, max_depth: Option<u32>) -> Result<Vec<ScanResult>> {
    crate::security::scanner::scan_directory(&path, max_depth.unwrap_or(5))
}

#[tauri::command]
pub fn security_quarantine_list(state: State<'_, Arc<Mutex<AppState>>>) -> Vec<QuarantineEntry> {
    let s = state.lock();

    s.quarantine.lock().list().to_vec()
}

#[tauri::command]
pub fn security_restore_file(state: State<'_, Arc<Mutex<AppState>>>, id: String) -> Result<()> {
    let s = state.lock();

    s.quarantine.lock().restore(&id)
}

#[tauri::command]
pub fn security_set_auto_defense(
    state: State<'_, Arc<Mutex<AppState>>>,
    enabled: bool,
) -> Result<()> {
    let s = state.lock();
    {
        let mut cfg = s.config.write();
        cfg.security.auto_defense = enabled;
        cfg.save()?;
    }
    Ok(())
}

/// Check file integrity against stored baselines.
#[tauri::command]
pub fn security_integrity_check() -> Result<Vec<IntegrityEvent>> {
    crate::security::integrity::check_integrity()
}

/// Save current file hashes as integrity baseline.
#[tauri::command]
pub fn security_integrity_save_baseline() -> Result<Vec<String>> {
    crate::security::integrity::save_baseline()
}

/// Scan for network anomalies.
#[tauri::command]
pub fn security_network_scan() -> Vec<NetworkAnomaly> {
    crate::security::network::detect_anomalies()
}

// ===========================================================================
// Modes
// ===========================================================================

#[tauri::command]
pub fn modes_get_active(state: State<'_, Arc<Mutex<AppState>>>) -> Mode {
    let s = state.lock();

    s.config.read().mode.clone().into()
}

#[tauri::command]
pub fn modes_set_mode(state: State<'_, Arc<Mutex<AppState>>>, mode: Mode) -> Result<()> {
    let s = state.lock();
    {
        let mut cfg = s.config.write();
        cfg.mode = mode.into();
        cfg.save()?;
    }
    Ok(())
}

// ===========================================================================
// Settings / i18n
// ===========================================================================

#[tauri::command]
pub fn settings_get(state: State<'_, Arc<Mutex<AppState>>>) -> SettingsDto {
    let s = state.lock();
    let cfg = s.config.read();
    SettingsDto {
        language: cfg.language.clone(),
        mode: format!("{:?}", cfg.mode).to_lowercase(),
        allow_autonomous: cfg.allow_autonomous,
        bypass_mode: cfg.bypass_mode,
        auto_defense: cfg.security.auto_defense,
        monitor: cfg.security.monitor,
        scanner_enabled: cfg.security.scanner_enabled,
        quarantine_auto_delete_days: cfg.security.quarantine_auto_delete_days,
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingsDto {
    pub language: String,
    pub mode: String,
    pub allow_autonomous: bool,
    pub bypass_mode: bool,
    pub auto_defense: bool,
    pub monitor: bool,
    pub scanner_enabled: bool,
    pub quarantine_auto_delete_days: u32,
}

#[tauri::command]
pub fn settings_set(state: State<'_, Arc<Mutex<AppState>>>, dto: SettingsDto) -> Result<()> {
    let s = state.lock();
    {
        let mut cfg = s.config.write();
        cfg.language = dto.language.clone();
        cfg.mode = match dto.mode.as_str() {
            "continuous" => OperatingMode::Continuous,
            _ => OperatingMode::OnDemand,
        };
        cfg.allow_autonomous = dto.allow_autonomous;
        cfg.bypass_mode = dto.bypass_mode;
        cfg.security.auto_defense = dto.auto_defense;
        cfg.security.monitor = dto.monitor;
        cfg.security.scanner_enabled = dto.scanner_enabled;
        cfg.security.quarantine_auto_delete_days = dto.quarantine_auto_delete_days;
        cfg.save()?;
    }
    i18n::set_locale(i18n::Locale::from_code(&dto.language));
    Ok(())
}

#[tauri::command]
pub fn i18n_get_locale() -> String {
    i18n::current().code().to_string()
}

#[tauri::command]
pub fn i18n_set_locale(locale: String) -> Result<()> {
    let l = i18n::Locale::from_code(&locale);
    i18n::set_locale(l);
    Ok(())
}

#[tauri::command]
pub fn i18n_translate(key: String) -> String {
    i18n::t(&key)
}

// ===========================================================================
// System
// ===========================================================================

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub fn app_quit(app: tauri::AppHandle) {
    app.exit(0);
}

// ===========================================================================
// v0.3 — Agent loop (computer-use co-owner)
// ===========================================================================

/// Kick off an agent run: the AI autonomously decides which tools to call,
/// dispatches them through the safety policy, and returns the final reply.
///
/// Emits `agent://tool_call`, `agent://tool_result`, `agent://confirmation`,
/// `agent://done`, and `agent://error` events to the frontend.
#[tauri::command]
pub async fn ai_agent_run(
    state: State<'_, Arc<Mutex<AppState>>>,
    app: tauri::AppHandle,
    params: crate::ai::agent::AgentRunParams,
) -> Result<String> {
    let state_inner = state.inner().clone();
    crate::ai::agent::run_agent_loop(state_inner, app, params).await
}

/// Trip the kill switch — aborts every running agent loop on its next iteration.
#[tauri::command]
pub fn safety_trip_kill_switch() -> Result<()> {
    crate::computer::kill_switch::trip();
    Ok(())
}

/// Reset the kill switch — allows new agent runs.
#[tauri::command]
pub fn safety_reset_kill_switch() -> Result<()> {
    crate::computer::kill_switch::reset();
    Ok(())
}

/// Returns the current kill-switch state.
#[tauri::command]
pub fn safety_kill_switch_status() -> bool {
    crate::computer::kill_switch::is_tripped()
}

/// Returns the current rate-limiter state: tokens available + max capacity.
#[tauri::command]
pub fn safety_rate_limiter_status() -> RateLimiterStatus {
    RateLimiterStatus {
        available_tokens: crate::computer::rate_limiter::available_tokens(),
        capacity: 30.0,
    }
}

#[derive(Debug, Serialize)]
pub struct RateLimiterStatus {
    pub available_tokens: f64,
    pub capacity: f64,
}

/// Reset the rate limiter to full (used after explicit user override).
#[tauri::command]
pub fn safety_rate_limiter_reset() -> Result<()> {
    crate::computer::rate_limiter::reset();
    Ok(())
}

/// Return the last N audit entries (newest first).
#[tauri::command]
pub fn audit_recent(
    state: State<'_, Arc<Mutex<AppState>>>,
    limit: Option<u32>,
) -> Result<Vec<crate::computer::audit::AuditEntry>> {
    let s = state.lock();
    let conn = s.memory.shared_conn();
    let conn = conn.lock();
    crate::computer::audit::recent(&conn, limit.unwrap_or(50))
}

/// Total audit log size.
#[tauri::command]
pub fn audit_count(state: State<'_, Arc<Mutex<AppState>>>) -> Result<u64> {
    let s = state.lock();
    let conn = s.memory.shared_conn();
    let conn = conn.lock();
    crate::computer::audit::count(&conn)
}

/// Wipe the audit log entirely (used by `aegis forget`).
#[tauri::command]
pub fn audit_wipe(state: State<'_, Arc<Mutex<AppState>>>) -> Result<()> {
    let s = state.lock();
    let conn = s.memory.shared_conn();
    let conn = conn.lock();
    crate::computer::audit::wipe(&conn)
}

/// Returns whether the built-in Aegis Cloud provider is preconfigured (i.e.
/// the env var `AEGIS_DEFAULT_API_KEY` or `ZAI_API_KEY` is set, or a key
/// was found in the OS keychain).
#[tauri::command]
pub fn aegis_cloud_preconfigured(state: State<'_, Arc<Mutex<AppState>>>) -> bool {
    let s = state.lock();
    let registry = s.providers.lock();
    registry.aegis_cloud_preconfigured()
}

/// Configure the Aegis Cloud provider with a user-supplied API key.
/// The key is stored in the OS keychain (with config.toml fallback).
#[tauri::command]
pub fn aegis_cloud_configure(
    state: State<'_, Arc<Mutex<AppState>>>,
    api_key: String,
) -> Result<()> {
    // Store in keychain under the "aegis-cloud" keyring user.
    if !api_key.trim().is_empty() {
        match keyring::Entry::new("aegis-ai", "aegis-cloud") {
            Ok(entry) => {
                if let Err(e) = entry.set_password(&api_key) {
                    tracing::warn!("aegis-cloud keyring write failed: {e}");
                }
            }
            Err(e) => tracing::warn!("aegis-cloud keyring entry creation failed: {e}"),
        }
    }
    // Inject into the live provider + persist a config entry so the UI shows
    // it as configured.
    {
        let s = state.lock();
        let registry = s.providers.lock();
        if let Some(p) = registry.get("aegis-cloud") {
            let creds = crate::ai::provider::ProviderCreds {
                api_key: Some(api_key.clone()),
                base_url: Some(crate::ai::providers::aegis_cloud::DEFAULT_BASE_URL.into()),
                model: Some(crate::ai::providers::aegis_cloud::DEFAULT_MODEL.into()),
                extra: Default::default(),
            };
            p.set_creds(creds);
        }
        let mut config = s.config.write();
        config.providers.credentials.insert(
            "aegis-cloud".into(),
            crate::config::ProviderCredentials {
                api_key: Some(api_key),
                base_url: Some(crate::ai::providers::aegis_cloud::DEFAULT_BASE_URL.into()),
                model: Some(crate::ai::providers::aegis_cloud::DEFAULT_MODEL.into()),
                enabled: true,
                extra: Default::default(),
            },
        );
        // Make Aegis Cloud the active provider if none is selected.
        if config.active_provider.is_none() {
            config.active_provider = Some("aegis-cloud".into());
        }
        config.save()?;
    }
    let router = {
        let s = state.lock();
        s.router.clone()
    };
    router.refresh();
    Ok(())
}

/// Test the built-in Aegis Cloud provider by sending a minimal ping.
#[tauri::command]
pub async fn aegis_cloud_test(state: State<'_, Arc<Mutex<AppState>>>) -> Result<()> {
    let providers = {
        let s = state.lock();

        (*s.providers.lock()).clone()
    };
    let provider = providers
        .get("aegis-cloud")
        .ok_or_else(|| AegisError::AiNotConfigured("aegis-cloud provider not registered".into()))?;
    provider.ping().await
}

/// Return the list of all available tool specs (sent to the AI on each call).
#[tauri::command]
pub fn agent_list_tools() -> serde_json::Value {
    crate::ai::tools::specs_as_json()
}

// ===========================================================================
// v0.4 — Bypass Mode
// ===========================================================================

/// Returns whether bypass mode is currently enabled.
#[tauri::command]
pub fn bypass_mode_status(state: State<'_, Arc<Mutex<AppState>>>) -> bool {
    let s = state.lock();
    s.config.read().bypass_mode
}

/// Enable bypass mode — the AI will skip the safety confirmation prompt for
/// all medium- and high-risk actions, except for the irrevocable hard-deny
/// list (rm -rf /, mkfs, dd to device, sudo to root, credential dumpers,
/// reverse shells, kernel modules, etc.).
///
/// The audit log still records every action.
#[tauri::command]
pub fn bypass_mode_enable(state: State<'_, Arc<Mutex<AppState>>>) -> Result<()> {
    {
        let s = state.lock();
        let mut cfg = s.config.write();
        cfg.bypass_mode = true;
        cfg.save()?;
    }
    // Refresh the router so the change takes effect immediately.
    let router = {
        let s = state.lock();
        s.router.clone()
    };
    router.refresh();
    tracing::warn!(
        "bypass mode ENABLED by user — AI will skip safety confirmations except for the irrevocable hard-deny list"
    );
    Ok(())
}

/// Disable bypass mode — back to normal safety policy.
#[tauri::command]
pub fn bypass_mode_disable(state: State<'_, Arc<Mutex<AppState>>>) -> Result<()> {
    {
        let s = state.lock();
        let mut cfg = s.config.write();
        cfg.bypass_mode = false;
        cfg.save()?;
    }
    let router = {
        let s = state.lock();
        s.router.clone()
    };
    router.refresh();
    tracing::info!("bypass mode DISABLED — normal safety policy restored");
    Ok(())
}

// ===========================================================================
// v0.4 — AI model catalog
// ===========================================================================

/// Returns the full AI model catalog (all providers + all models).
/// Use this to populate the Providers UI's model picker.
#[tauri::command]
pub fn ai_list_models() -> serde_json::Value {
    serde_json::json!({
        "providers": crate::ai::catalog::providers(),
        "models": crate::ai::catalog::models(),
        "provider_count": crate::ai::catalog::provider_count(),
        "model_count": crate::ai::catalog::model_count(),
    })
}

/// Returns the catalog entries for a single provider.
#[tauri::command]
pub fn ai_models_for_provider(
    provider_id: String,
) -> Vec<&'static crate::ai::catalog::CatalogModel> {
    crate::ai::catalog::models_for_provider(&provider_id)
}

// ===========================================================================
// v0.4 — Skills
// ===========================================================================

/// Returns the list of all available skills.
#[tauri::command]
pub fn skills_list() -> &'static [crate::ai::skills::Skill] {
    crate::ai::skills::all_skills()
}

/// Returns the currently active skill id (read from the sidecar file).
#[tauri::command]
pub fn skills_active() -> Option<String> {
    let path = crate::config::AppConfig::data_dir().join("active_skill");
    std::fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Set the active skill by id. The agent loop will inject its prompt fragment.
#[tauri::command]
pub fn skills_set(id: String) -> Result<()> {
    if crate::ai::skills::find(&id).is_none() {
        return Err(AegisError::Config(format!("unknown skill: {id}")));
    }
    let path = crate::config::AppConfig::data_dir().join("active_skill");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&path, &id)
        .map_err(|e| AegisError::Io(format!("failed to write active_skill: {e}")))?;
    Ok(())
}

// ===========================================================================
// v0.5 — Voice I/O (STT / TTS / Push-to-talk)
// ===========================================================================

/// Transcribe an audio blob via the configured STT backend.
///
/// The frontend should send raw bytes captured from the microphone plus a
/// MIME type (`audio/wav`, `audio/mpeg`, `audio/webm`, …). The command
/// returns a [`Transcript`] which includes the recognized text, the wake
/// word detection flag, and the backend name.
#[tauri::command]
pub async fn voice_transcribe(audio_b64: String, mime: String) -> Result<crate::voice::Transcript> {
    use base64::Engine as _;
    let audio = base64::engine::general_purpose::STANDARD
        .decode(audio_b64.as_bytes())
        .map_err(|e| AegisError::Internal(format!("base64 decode failed: {e}")))?;
    let stt = crate::voice::stt::default_stt();
    stt.transcribe(&audio, &mime).await
}

/// Synthesize speech from text. Writes the resulting audio file to a temp
/// path (or `opts.out_path` if provided) and returns the file path + MIME.
///
/// The frontend can play the result with `<audio src="file://…">`.
#[tauri::command]
pub async fn voice_speak(
    text: String,
    opts: Option<crate::voice::TtsOptions>,
) -> Result<serde_json::Value> {
    let opts = opts.unwrap_or_default();
    let tts = crate::voice::tts::default_tts();
    let speech = tts.synthesize(&text, &opts).await?;
    Ok(serde_json::to_value(&speech)?)
}

/// Return the current push-to-talk hotkey state.
#[tauri::command]
pub fn voice_ptt_state(state: State<'_, Arc<Mutex<AppState>>>) -> serde_json::Value {
    let s = state.lock();
    serde_json::json!({
        "state": match s.hotkey.state() {
            crate::voice::PushToTalkState::Idle => "idle",
            crate::voice::PushToTalkState::Recording => "recording",
        },
        "hotkey": s.hotkey.hotkey(),
        "registered": s.hotkey.is_registered(),
    })
}

/// Update the push-to-talk hotkey. Re-registers the global shortcut.
#[tauri::command]
pub fn voice_ptt_set_hotkey(
    state: State<'_, Arc<Mutex<AppState>>>,
    app: tauri::AppHandle,
    hotkey: String,
) -> Result<serde_json::Value> {
    // Parse the new hotkey before clearing the old one so we surface a
    // config error to the frontend without losing the existing registration.
    let _shortcut: tauri_plugin_global_shortcut::Shortcut = hotkey
        .parse()
        .map_err(|e| AegisError::Config(format!("invalid hotkey '{hotkey}': {e}")))?;
    let s = state.lock();
    if let Err(e) = crate::voice::hotkey::unregister(&app, &s.hotkey) {
        tracing::warn!("failed to unregister previous PTT hotkey: {e}");
    }
    s.hotkey.set_hotkey(&hotkey);
    crate::voice::hotkey::register(&app, &s.hotkey)?;
    Ok(serde_json::json!({
        "hotkey": hotkey,
        "registered": s.hotkey.is_registered(),
    }))
}

// ===========================================================================
// v0.5 — Calendar (CalDAV)
// ===========================================================================

/// Returns today's calendar events. Empty vec if no CalDAV server configured.
#[tauri::command]
pub async fn calendar_list_today(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<crate::calendar::CalendarEvent>> {
    let client = {
        let s = state.lock();
        let cal = s.calendar.lock();
        cal.clone()
    };
    client.today().await
}

/// Configure the CalDAV server connection. Stored in `config.toml`.
#[tauri::command]
pub fn calendar_configure(
    state: State<'_, Arc<Mutex<AppState>>>,
    cfg: crate::calendar::CalendarConfig,
) -> Result<()> {
    let new_client = crate::calendar::CalendarClient::new(cfg.clone())?;
    {
        let s = state.lock();
        *s.calendar.lock() = new_client;
    }
    tracing::info!("calendar configured: url={} user={}", cfg.url, cfg.username);
    Ok(())
}

/// Classify a natural-language message and dispatch it as a calendar intent.
/// For list-style intents, fetches events from CalDAV. For schedule_meeting,
/// fetches today's events so the AI can detect conflicts.
#[tauri::command]
pub async fn calendar_dispatch_intent(
    state: State<'_, Arc<Mutex<AppState>>>,
    message: String,
) -> Result<crate::calendar::CalendarDispatchResult> {
    let client = {
        let s = state.lock();
        let cal = s.calendar.lock();
        cal.clone()
    };
    crate::calendar::dispatch_calendar_intent(&message, &client).await
}

// ===========================================================================
// v0.6 — Web access (real web_search + http_fetch readability)
// ===========================================================================

/// Run a real web search via DuckDuckGo's HTML endpoint. No API key needed.
/// Returns up to 8 hits with title, URL, and snippet.
#[tauri::command]
pub async fn web_search(query: String) -> Result<Vec<crate::ai::web::SearchResult>> {
    crate::ai::web::web_search(&query).await
}

/// Fetch a URL and return its readable text (HTML boilerplate stripped).
/// Caps the response at 32 KB. Useful for ingesting articles / docs.
#[tauri::command]
pub async fn web_fetch(url: String) -> Result<String> {
    crate::ai::web::fetch_readable(&url).await
}

/// Fetch a URL and return raw HTML/JSON (no readability extraction).
/// Caps the response at 256 KB. Use this for JSON APIs or raw downloads.
#[tauri::command]
pub async fn web_fetch_raw(
    url: String,
    method: Option<String>,
    body: Option<String>,
) -> Result<serde_json::Value> {
    let method = method.unwrap_or_else(|| "GET".into());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| AegisError::Internal(format!("http client build failed: {e}")))?;
    let m = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|e| AegisError::Config(format!("invalid method '{method}': {e}")))?;
    let mut req = client.request(m, &url);
    if let Some(b) = body {
        req = req.body(b);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| AegisError::Ai(format!("http fetch failed: {e}")))?;
    let status = resp.status().as_u16();
    let text = resp
        .text()
        .await
        .map_err(|e| AegisError::Ai(format!("http fetch body read failed: {e}")))?;
    let len = text.len();
    let truncated = if len > 256 * 1024 {
        let mut t = text[..256 * 1024].to_string();
        t.push_str("\n...[truncated]");
        t
    } else {
        text
    };
    Ok(serde_json::json!({
        "status": status,
        "body": truncated,
        "len": len,
        "url": url,
    }))
}

// ===========================================================================
// v0.6 — Memory: entity extraction
// ===========================================================================

/// Run entity extraction over the last N messages of a conversation and
/// persist any new entities to the knowledge base. Returns the count of
/// new facts stored.
#[tauri::command]
pub fn memory_extract_entities(
    state: State<'_, Arc<Mutex<AppState>>>,
    conversation_id: String,
    limit: Option<u32>,
) -> Result<usize> {
    let s = state.lock();
    let messages = s.memory.conversations.messages(&conversation_id)?;
    let limit = limit.unwrap_or(50) as usize;
    let texts: Vec<String> = messages
        .into_iter()
        .rev()
        .take(limit)
        .map(|m| m.content)
        .collect();
    let n = crate::memory::entities::extract_and_store(&s.memory, &texts)?;
    if n > 0 {
        tracing::info!("extracted {n} new entities from conversation {conversation_id}");
    }
    Ok(n)
}

/// Return the database encryption status (Phase 2.5 stub).
#[tauri::command]
pub fn memory_encryption_status() -> serde_json::Value {
    let status = crate::memory::encryption::status();
    serde_json::json!({
        "status": match status {
            crate::memory::EncryptionStatus::NotSupported => "not_supported",
            crate::memory::EncryptionStatus::Disabled => "disabled",
            crate::memory::EncryptionStatus::Enabled => "enabled",
        },
        "supported": crate::memory::encryption::is_supported(),
    })
}

// ===========================================================================
// v0.6 — Security: YARA rules + audit log export
// ===========================================================================

/// Return the list of loaded YARA rules. Empty if the user hasn't dropped
/// any `.yar` / `.yara` files into their data directory.
#[tauri::command]
pub fn yara_list() -> Result<Vec<crate::security::yara::YaraRule>> {
    crate::security::yara::load_all()
}

/// Ensure the YARA rules directory exists. Called at boot so the user can
/// drop rule files into it without manually creating the directory.
#[tauri::command]
pub fn yara_ensure_dir() -> Result<()> {
    crate::security::yara::ensure_dir()
}

/// Export the audit log as JSON (Phase 4.3 — GDPR data export).
/// Returns the last N entries as a JSON array. The frontend can save the
/// result to a file via the Tauri `dialog` plugin.
#[tauri::command]
pub fn audit_export(
    state: State<'_, Arc<Mutex<AppState>>>,
    limit: Option<u32>,
    format: Option<String>,
) -> Result<serde_json::Value> {
    let s = state.lock();
    let conn = s.memory.shared_conn();
    let conn = conn.lock();
    let limit = limit.unwrap_or(10_000);
    let entries = crate::computer::audit::recent(&conn, limit)?;
    match format.as_deref().unwrap_or("json") {
        "json" => Ok(serde_json::to_value(&entries)?),
        "csv" => {
            let mut wtr = csv_writer();
            for e in &entries {
                let _ = wtr.write_record(&[
                    e.id.to_string(),
                    e.ts_ms.to_string(),
                    e.conversation_id.clone().unwrap_or_default(),
                    e.agent_run_id.clone().unwrap_or_default(),
                    e.tool_name.clone(),
                    e.arguments_json.clone(),
                    e.result_json.clone(),
                    e.outcome.clone(),
                    e.duration_ms.to_string(),
                ]);
            }
            Ok(serde_json::json!({
                "format": "csv",
                "rows": entries.len(),
                "csv": wtr.to_string(),
            }))
        }
        _ => Err(AegisError::Config(format!(
            "unknown export format: {format:?}"
        ))),
    }
}

/// Export all conversations + messages as a single JSON document. This is
/// the "GDPR data export" endpoint (Phase 4.3) — the user can request a
/// full dump of their data for portability or deletion review.
#[tauri::command]
pub fn memory_export_all(state: State<'_, Arc<Mutex<AppState>>>) -> Result<serde_json::Value> {
    let s = state.lock();
    let convs = s.memory.conversations.list(100_000)?;
    let mut out = Vec::with_capacity(convs.len());
    for c in &convs {
        let msgs = s.memory.conversations.messages(&c.id)?;
        out.push(serde_json::json!({
            "conversation": c,
            "messages": msgs,
        }));
    }
    Ok(serde_json::json!({
        "exported_at_ms": time::OffsetDateTime::now_utc().unix_timestamp() * 1000,
        "version": env!("CARGO_PKG_VERSION"),
        "conversation_count": convs.len(),
        "data": out,
    }))
}

/// Wipe all user data (GDPR "right to be forgotten"). Drops conversations,
/// activities, knowledge, embeddings, audit log, and integrity baselines.
/// Settings and provider credentials are NOT wiped (use the Providers UI).
#[tauri::command]
pub fn memory_forget_all(state: State<'_, Arc<Mutex<AppState>>>) -> Result<()> {
    let s = state.lock();
    let conn = s.memory.shared_conn();
    let conn = conn.lock();
    conn.execute_batch(
        "DELETE FROM messages;
         DELETE FROM conversations;
         DELETE FROM activities;
         DELETE FROM knowledge;
         DELETE FROM knowledge_embeddings;
         DELETE FROM events;
         DELETE FROM integrity_baselines;
         DELETE FROM audit_log;",
    )?;
    tracing::info!("all user data wiped (memory_forget_all)");
    Ok(())
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Tiny CSV writer — avoids pulling in the `csv` crate for one endpoint.
fn csv_writer() -> CsvWriter {
    CsvWriter::default()
}

#[derive(Default)]
struct CsvWriter {
    buf: String,
}

impl CsvWriter {
    fn write_record(&mut self, fields: &[String]) -> std::io::Result<()> {
        let mut row = String::new();
        for (i, f) in fields.iter().enumerate() {
            if i > 0 {
                row.push(',');
            }
            let needs_quote = f.contains(',') || f.contains('"') || f.contains('\n');
            if needs_quote {
                row.push('"');
                row.push_str(&f.replace('"', "\"\""));
                row.push('"');
            } else {
                row.push_str(f);
            }
        }
        row.push('\n');
        self.buf.push_str(&row);
        Ok(())
    }
}

impl std::fmt::Display for CsvWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.buf)
    }
}

// ===========================================================================
// v0.7 — Phase 4.2: Sandbox
// ===========================================================================

/// DTO for sandbox policy state.
#[derive(Debug, Serialize, Deserialize)]
pub struct SandboxPolicyDto {
    pub enabled: bool,
    pub allowed_dirs: Vec<String>,
    pub allow_home_subdirs: bool,
}

impl From<&SandboxPolicy> for SandboxPolicyDto {
    fn from(p: &SandboxPolicy) -> Self {
        Self {
            enabled: p.enabled,
            allowed_dirs: p.allowed_dirs.clone(),
            allow_home_subdirs: p.allow_home_subdirs,
        }
    }
}

/// Return the current sandbox policy.
#[tauri::command]
pub fn sandbox_status(
    state: State<'_, SharedState>,
) -> std::result::Result<SandboxPolicyDto, String> {
    let s = state.lock();
    let policy = s.sandbox.lock();
    Ok(SandboxPolicyDto::from(&*policy))
}

/// Enable or disable the sandbox.
#[tauri::command]
pub fn sandbox_set_enabled(
    state: State<'_, SharedState>,
    enabled: bool,
) -> std::result::Result<(), String> {
    let s = state.lock();
    let mut policy = s.sandbox.lock();
    policy.enabled = enabled;
    tracing::info!("sandbox: enabled={enabled}");
    Ok(())
}

/// Add a directory to the sandbox allow-list.
#[tauri::command]
pub fn sandbox_add_dir(
    state: State<'_, SharedState>,
    dir: String,
) -> std::result::Result<(), String> {
    let s = state.lock();
    let mut policy = s.sandbox.lock();
    policy.add_allowed_dir(dir);
    Ok(())
}

/// Remove a directory from the sandbox allow-list.
#[tauri::command]
pub fn sandbox_remove_dir(
    state: State<'_, SharedState>,
    dir: String,
) -> std::result::Result<(), String> {
    let s = state.lock();
    let mut policy = s.sandbox.lock();
    policy.remove_allowed_dir(&dir);
    Ok(())
}

// ===========================================================================
// v0.7 — Phase 4.3: Telemetry
// ===========================================================================

/// DTO for telemetry summary.
#[derive(Debug, Serialize, Deserialize)]
pub struct TelemetrySummaryDto {
    pub enabled: bool,
    pub prompted: bool,
    pub pending_count: usize,
    pub install_id: String,
}

impl From<security::telemetry::TelemetrySummary> for TelemetrySummaryDto {
    fn from(s: security::telemetry::TelemetrySummary) -> Self {
        Self {
            enabled: s.enabled,
            prompted: s.prompted,
            pending_count: s.pending_count,
            install_id: s.install_id,
        }
    }
}

/// Return the current telemetry summary.
#[tauri::command]
pub fn telemetry_status(
    state: State<'_, SharedState>,
) -> std::result::Result<TelemetrySummaryDto, String> {
    let s = state.lock();
    let cfg = s.telemetry.lock();
    Ok(TelemetrySummaryDto::from(cfg.summary()))
}

/// Opt in to telemetry collection.
#[tauri::command]
pub fn telemetry_opt_in(state: State<'_, SharedState>) -> std::result::Result<(), String> {
    let s = state.lock();
    let mut cfg = s.telemetry.lock();
    cfg.opt_in();
    Ok(())
}

/// Opt out of telemetry collection.
#[tauri::command]
pub fn telemetry_opt_out(state: State<'_, SharedState>) -> std::result::Result<(), String> {
    let s = state.lock();
    let mut cfg = s.telemetry.lock();
    cfg.opt_out();
    Ok(())
}
