# Architecture

This document describes the high-level architecture of Aegis AI.

## Overview

Aegis AI is a Tauri 2.0 desktop application. The Rust backend exposes a set
of Tauri commands that the React frontend invokes via the standard Tauri
IPC bridge. All state lives in two places:

1. **In-memory**: `AppState` (held in `Arc<Mutex<AppState>>` and managed by
   Tauri). Subsystems hold `Arc` clones to shared resources (the AI router,
   the memory store, the config store).
2. **On-disk**: `config.toml` for preferences and `aegis.db` (SQLite) for
   conversations, activities, knowledge, and events.

```
┌───────────────────────────────────────────────────────────────┐
│                       React Frontend                          │
│  (Chat, Providers, Memory, Security, Modes, Settings)         │
└───────────────────────────┬───────────────────────────────────┘
                            │ invoke<…>("command_name", …)
                            ▼
┌───────────────────────────────────────────────────────────────┐
│                     Tauri IPC Commands                        │
│              src-tauri/src/commands.rs (25+ commands)         │
└───────────────────────────┬───────────────────────────────────┘
                            │ calls into
                            ▼
┌───────────────────────────────────────────────────────────────┐
│                    AppState (Arc<Mutex<_>>)                    │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐  │
│  │ ConfigStore│ │ AI Router  │ │ MemoryStore│ │ ProviderReg│  │
│  └────────────┘ └────────────┘ └────────────┘ └────────────┘  │
└───────────────────────────┬───────────────────────────────────┘
                            │ owns / drives
              ┌─────────────┼─────────────┬─────────────┐
              ▼             ▼             ▼             ▼
        ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐
        │   ai/    │  │ computer/│  │ security/│  │  memory/ │
        │ (20+ pr) │  │ (safety) │  │ (defense)│  │ (SQLite) │
        └──────────┘  └──────────┘  └──────────┘  └──────────┘
```

## Module responsibilities

### `ai/` — Multi-provider AI routing

- `provider.rs`: defines the `Provider` trait, `ChatRequest` / `ChatResponse`
  types, and a `ProviderRegistry` that pre-populates 20+ providers.
- `router.rs`: `AiRouter` resolves the active provider from config, injects
  the latest credentials, and forwards chat / streaming requests.
- `providers/openai_compat.rs`: shared HTTP client for every provider that
  exposes the OpenAI `/v1/chat/completions` shape (most of them).
- `providers/openai.rs`, `anthropic.rs`, `gemini.rs`, `ollama.rs`: bespoke
  clients for providers whose API shape differs.

### `computer/` — Computer-use agent

- `safety.rs`: 5-level risk classifier with whitelist + denylist. Every
  destructive action returns a `SafetyDecision::RequireConfirmation` with a
  token.
- `commands.rs`, `files.rs`, `apps.rs`: shell exec, file I/O, app launch.
- `automation.rs`: declarative `AutoAction` enum (mouse, keyboard, sleep)
  consumed by `auto_perform`. v0.1 has stub implementations; Phase 2 adds
  real `enigo` integration.
- `screen.rs`: screenshot + OCR. v0.1 returns a placeholder PNG.

### `security/` — Defense subsystem

- `monitor.rs`: polls processes every 15s. On Linux reads `/proc/<pid>/cmdline`;
  on Windows uses `CreateToolhelp32Snapshot`. Each process is matched against
  the user-configured threat signatures.
- `defender.rs`: consumes threats from a shared channel, escalates response
  by severity: `Notify` → `Quarantine` → `Kill`.
- `scanner.rs`: on-demand file hash scanner against a built-in list of
  known-bad SHA-256 hashes (EICAR test file + sample sigs).
- `quarantine.rs`: copy-then-delete file quarantine with restore / delete.

### `memory/` — Persistent store

- `store.rs`: opens a single SQLite connection (shared across sub-stores via
  `Arc<Mutex<Connection>>`). Runs schema migrations on boot.
- `conversation.rs`, `activity.rs`, `knowledge.rs`: domain-specific stores.

### `modes/` — Operational modes

- `continuous.rs`: 60-second heartbeat task. Phase 3 will wire it to a
  richer event bus (file watchers, calendar, security events).
- `ondemand.rs`: AI dormant until called. Security monitor still runs.

### `i18n/` — Translations

Static table of (English, Vietnamese) tuples for ~80 keys. Phase 4 will
migrate to `fluent-bundle` for proper pluralization.

## Data flow: a chat request

1. User types in the Chat component and presses Enter.
2. Frontend calls `ai_chat({ user_message, conversation_id })` via
   `invoke()`.
3. Rust `ai_chat` command:
   - Creates a conversation if `conversation_id` is null.
   - Persists the user message to SQLite.
   - Loads the conversation history.
   - Prepends a system prompt.
   - Calls `AiRouter::chat()`, which resolves the active provider and
     forwards the request.
   - Persists the assistant reply to SQLite.
   - Logs an `ActivityRecord` to the activity table.
4. Response is returned to the frontend, which renders the bubble.

## Data flow: a security escalation

1. `monitor::start` polls processes every 15s.
2. Each process's command line is matched against threat signatures.
3. Matches are pushed to a shared `INCOMING` channel.
4. `defender::start` drains the channel every 2s.
5. For each threat:
   - Always emits a Tauri event `security://threat` (UI shows toast).
   - If `auto_defense` is enabled and severity ≥ Medium: quarantines the
     binary path of the offending process.
   - If severity is Critical: also kills the process.
6. Every defensive action is logged to `EVENTS` and surfaced in the UI.

## Configuration

`AppConfig` (in `config.rs`) is loaded from `config.toml` on boot. The
`ConfigStore` wraps it in `RwLock` for cheap reads and atomic writes.
`save()` re-serializes to disk after every mutation.

Subsystems that need to react to config changes do so via `ConfigStore::read()`
at the point of use — there is no event bus for config changes in v0.1.

## Adding a new AI provider

See [PROVIDERS.md](PROVIDERS.md) for the step-by-step guide.
