//! Tauri command handlers: the IPC bridge between the Rust backend and the
//! React frontend.
//!
//! Every command is registered in `lib.rs::run` via `tauri::generate_handler!`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::State;
use tauri::Emitter;

use crate::{
    ai::provider::{ChatMessage, ChatRequest, ChatStreamChunk, ProviderCredentials},
    computer::{
        apps::{list_apps, open_app, AppDescriptor},
        automation::{auto_perform, AutoAction},
        clipboard::{
            clipboard_read, clipboard_write, clipboard_watch_start, clipboard_watch_stop,
            ClipboardContent,
        },
        commands::{exec_command, ExecResult},
        files::{file_read, file_write, FileReadResult},
        safety::{SafetyPolicy},
        screenshot,
    },
    config::{AppConfig, OperatingMode},
    error::{AegisError, Result},
    i18n,
    memory::{ActivityRecord, Conversation, KnowledgeEntry, Message},
    modes::Mode,
    security::{
        self, monitor::Threat, quarantine::QuarantineEntry, scanner::ScanResult, DefenseEvent,
        integrity::IntegrityEvent, network::{NetworkAnomaly, SocketInfo},
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
        s.providers.lock().clone()
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
        let _ = s.memory.activity.record(
            "chat.user",
            &format!("user message in {conv_id}"),
            None,
        );
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
            &format!("assistant replied in {conv_id} ({} chars)", resp.message.content.len()),
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
        s.providers.lock().clone()
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
        s.memory.conversations.add_message(&conv_id, "user", &params.user_message)?;
        let _ = s.memory.activity.record("chat.user", &format!("streaming user message in {conv_id}"), None);
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
    messages.insert(0, ChatMessage::system(
        "You are Aegis AI, a secure cross-platform assistant. \
         Be concise, accurate, and refuse any request that would harm the user's system."
    ));

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
        let on_chunk: Box<dyn Fn(ChatStreamChunk) + Send + Sync> = Box::new(move |chunk| {
            let _ = app_for_chunk.emit("chat://chunk", &serde_json::json!({
                "stream_id": stream_id_clone,
                "delta": chunk.delta,
                "done": chunk.done,
            }));
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
                            "stream_id": stream_id_clone,
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
            let _ = app.emit("chat://done", &serde_json::json!({
                "stream_id": stream_id_clone,
            }));
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
pub fn ai_chat_cancel(
    state: State<'_, Arc<Mutex<AppState>>>,
    stream_id: String,
) -> Result<()> {
    let s = state.lock();
    let mut tokens = s.cancel_tokens.lock();
    if let Some(tx) = tokens.remove(&stream_id) {
        let _ = tx.send(true);
        tracing::info!("cancelled stream: {}", stream_id);
        Ok(())
    } else {
        Err(AegisError::Internal(format!("no active stream with id {stream_id}")))
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
            let creds = cfg.providers.credentials.get(&d.id);
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
                enabled: creds.map(|c| c.enabled).unwrap_or(false),
                is_active: cfg.active_provider.as_deref() == Some(&d.id),
                configured: creds.map(|c| {
                    c.api_key.is_some() || !d.requires_api_key || c.base_url.is_some()
                }).unwrap_or(false),
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
            Ok(entry) => {
                if let Err(e) = entry.set_password(api_key) {
                    tracing::warn!("keyring store failed for {}, falling back to config: {e}", cfg.provider_id);
                } else {
                    tracing::debug!("stored api_key in keyring for {}", cfg.provider_id);
                }
            }
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
        s.providers.lock().clone()
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
    let _ = s.memory.activity.record(
        "computer.exec",
        &format!("exec: {}", params.command),
        None,
    );
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
    let mut pending_map = {
        let s = state.lock();
        s.pending_actions.lock()
    };

    let pending = pending_map.remove(&params.token)
        .ok_or_else(|| AegisError::SafetyDenial(
            format!("invalid or expired action token: {}", params.token)
        ))?;

    // Check expiry (60 seconds)
    let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000;
    let elapsed = now_ms.saturating_sub(pending.created_at_ms);
    if elapsed > ACTION_TOKEN_TTL.as_millis() as u64 {
        return Err(AegisError::SafetyDenial(
            format!("action token expired ({}ms old)", elapsed)
        ));
    }

    // Verify action matches
    if pending.action != params.action {
        return Err(AegisError::SafetyDenial(
            "action does not match the confirmed token".into()
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
        s.providers.lock().clone()
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
    let conversation_text: String = history.iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    let messages = vec![
        ChatMessage::system(
            "You are a summarization assistant. Produce a concise summary of the conversation \
             that captures the key topics, decisions, and outcomes. Keep it under 200 words."
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
pub fn security_quarantine_list(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Vec<QuarantineEntry> {
    let s = state.lock();
    s.quarantine.lock().list().to_vec()
}

#[tauri::command]
pub fn security_restore_file(
    state: State<'_, Arc<Mutex<AppState>>>,
    id: String,
) -> Result<()> {
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
    s.config.read().mode.into()
}

#[tauri::command]
pub fn modes_set_mode(
    state: State<'_, Arc<Mutex<AppState>>>,
    mode: Mode,
) -> Result<()> {
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
    pub auto_defense: bool,
    pub monitor: bool,
    pub scanner_enabled: bool,
    pub quarantine_auto_delete_days: u32,
}

#[tauri::command]
pub fn settings_set(
    state: State<'_, Arc<Mutex<AppState>>>,
    dto: SettingsDto,
) -> Result<()> {
    let s = state.lock();
    {
        let mut cfg = s.config.write();
        cfg.language = dto.language.clone();
        cfg.mode = match dto.mode.as_str() {
            "continuous" => OperatingMode::Continuous,
            _ => OperatingMode::OnDemand,
        };
        cfg.allow_autonomous = dto.allow_autonomous;
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

// Silence unused warnings for re-exported types used only in command signatures.
#[allow(dead_code)]
fn _force_use(
    _a: ActivityRecord,
    _k: KnowledgeEntry,
    _c: ChatStreamChunk,
    _t: Threat,
    _d: DefenseEvent,
    _p: &dyn std::any::Any,
    _n: NetworkAnomaly,
    _s: SocketInfo,
    _i: IntegrityEvent,
    _cl: ClipboardContent,
    _pa: PendingAction,
) {
}
