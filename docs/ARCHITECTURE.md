# Architecture

High-level architecture of Aegis AI.

---

## Overview

Aegis AI is a Tauri 2.0 desktop app. The Rust backend exposes Tauri commands that the React frontend invokes via IPC. State lives in:

1. **In-memory**: `AppState` held in `Arc<Mutex<AppState>>` — subsystems hold `Arc` clones to shared resources
2. **On-disk**: `config.toml` for preferences, `aegis.db` (SQLite) for conversations, activities, knowledge, events

```
┌───────────────────────────────────────────────────────────────┐
│                       React Frontend                          │
│  (Chat, Providers, Memory, Security, Modes, Settings)         │
└───────────────────────────┬───────────────────────────────────┘
                            │ invoke("command_name", …)
                            ▼
┌───────────────────────────────────────────────────────────────┐
│                     Tauri IPC Commands                        │
│              src-tauri/src/commands.rs (50+ commands)         │
└───────────────────────────┬───────────────────────────────────┘
                            │ calls into
                            ▼
┌───────────────────────────────────────────────────────────────┐
│                    AppState (Arc<Mutex<_>>)                    │
│  ConfigStore │ AI Router │ MemoryStore │ ProviderRegistry      │
└───────────────────────────┬───────────────────────────────────┘
                            │ owns / drives
              ┌─────────────┼─────────────┬─────────────┐
              ▼             ▼             ▼             ▼
        ai/ (90+      computer/    security/    memory/
        providers)    (safety)     (defense)    (SQLite)
```

---

## Module Responsibilities

### `ai/` — Multi-provider AI routing

- `provider.rs`: `Provider` trait, `ChatRequest`/`ChatResponse`, `ProviderRegistry` (90+ providers)
- `router.rs`: resolves active provider, injects credentials, forwards requests
- `providers/openai_compat.rs`: shared HTTP client for OpenAI-compatible endpoints
- `providers/openai.rs`, `anthropic.rs`, `gemini.rs`, `ollama.rs`: bespoke clients

### `computer/` — Computer-use agent

- `safety.rs`: 5-level risk classifier + whitelist + denylist
- `commands.rs`, `files.rs`, `apps.rs`: shell exec, file I/O, app launch
- `automation.rs`: declarative `AutoAction` enum (mouse, keyboard, sleep)
- `screen.rs`: screenshot + OCR

### `security/` — Defense subsystem

- `monitor.rs`: polls processes every 15s, matches threat signatures
- `defender.rs`: notify → quarantine → kill escalation by severity
- `scanner.rs`: on-demand file hash scanner (SHA-256)
- `quarantine.rs`: copy-then-delete quarantine with restore/delete

### `memory/` — Persistent store

- `store.rs`: single SQLite connection (shared via `Arc<Mutex<Connection>>`), WAL mode
- `conversation.rs`, `activity.rs`, `knowledge.rs`: domain-specific stores

### `modes/` — Operational modes

- `continuous.rs`: 60-second heartbeat task
- `ondemand.rs`: AI dormant until called; security monitor still runs

### `i18n/` — Translations

Static 7-locale table (~80 keys). EN, VI, ES, FR, DE, JA, ZH-CN.

---

## Data Flow: Chat Request

1. User types → frontend calls `ai_chat({ user_message, conversation_id })`
2. Rust creates/persists conversation → loads history → prepends system prompt
3. `AiRouter::chat()` resolves active provider → forwards request
4. Response persisted to SQLite → activity logged → returned to frontend

## Data Flow: Security Escalation

1. `monitor::start` polls processes every 15s
2. Matches pushed to shared `INCOMING` channel
3. `defender::start` drains every 2s → emits Tauri event → escalates by severity
4. Every defensive action logged to `EVENTS` table

---

## Configuration

`AppConfig` loaded from `config.toml` on boot, wrapped in `RwLock` for cheap reads. `save()` re-serializes after every mutation.

## Adding a New AI Provider

See [PROVIDERS.md](PROVIDERS.md) for the step-by-step guide.
