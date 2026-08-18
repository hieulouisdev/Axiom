# Aegis AI — Developer Guide

**Version:** v0.9 | **Rust:** 1.97.1 | **Updated:** 2026-08

---

## 1. Development Setup

### Prerequisites

| Tool | Version | Notes |
|---|---|---|
| Rust | 1.97.1 | `rust-toolchain.toml` |
| Node.js | 20+ | LTS recommended |
| Tauri CLI | 2.0 | `cargo install tauri-cli --version "^2.0"` |

### Setup

```bash
git clone https://github.com/hieulouisdev/Axiom.git
cd Axiom
npm install
cargo tauri dev          # hot-reload Rust + React
```

### Useful Commands

```bash
cd src-tauri && cargo test              # all Rust tests
cd src-tauri && cargo test safety       # safety policy tests
cd src-tauri && cargo clippy -- -D warnings  # lint
cd src-tauri && cargo audit             # vulnerability check
npx tsc --noEmit                        # frontend type check
cargo tauri build                       # production build
```

### Environment Variables

| Variable | Description |
|---|---|
| `RUST_LOG` | Tracing filter (default: `info,aegis=debug`) |
| `AEGIS_DATA_DIR` | Override data directory (for testing) |

---

## 2. Architecture Overview

```
React Frontend ──invoke()──▶ Tauri Commands ──▶ AppState
                                                ├── ConfigStore
                                                ├── AiRouter (90+ providers)
                                                ├── MemoryStore (SQLite)
                                                └── ProviderRegistry
                                                      │
                    ┌─────────┬─────────┬──────────────┤
                    ▼         ▼         ▼              ▼
                 ai/    computer/  security/     memory/
               (90+ pr)  (safety)   (defense)    (SQLite)
```

### Key Subsystems

| Module | Responsibility |
|---|---|
| `ai/` | Multi-provider routing, `Provider` trait, `AiRouter`, `ProviderRegistry` |
| `computer/` | Safety policy, kill switch, rate limiter, audit, shell, file I/O, GUI |
| `security/` | Process monitor, auto-defender, file scanner, quarantine, YARA |
| `memory/` | SQLite stores: conversations, knowledge, embeddings, RAG |
| `voice/` | STT (Whisper), TTS (OS-native + ElevenLabs), push-to-talk |
| `modes/` | Continuous (60s heartbeat) vs on-demand |
| `i18n/` | 7-locale translation tables |

---

## 3. Adding a New AI Provider

See [PROVIDERS.md](PROVIDERS.md) for the full guide.

**TL;DR:**

- OpenAI-compatible → one file, ~15 lines via `openai_compat::make(descriptor(...))`
- Bespoke API → implement the `Provider` trait directly
- Register in `providers/mod.rs` and `ProviderRegistry::with_builtin`

---

## 4. Adding a New Agent Tool

1. **Define** in `ai/tools.rs` as an `AgentTool` enum variant
2. **Implement handler** in agent dispatch — must go through safety policy first
3. **Add to `agent_list_tools`** so the AI knows it's available
4. **Write tests**

```rust
AgentTool::MyNewTool { param } => {
    let check = safety::evaluate(&format!("my_new_tool: {param}"), &config)?;
    match check.decision {
        SafetyDecision::Allow => { /* proceed */ }
        SafetyDecision::RequireConfirmation { token, summary } => {
            return Err(AegisError::SafetyConfirmation { token, summary });
        }
        SafetyDecision::Deny { reason } => {
            return Err(AegisError::SafetyDeny { reason });
        }
    }
    // execute, audit, return
}
```

---

## 5. Adding a New Tauri Command

1. **Define** in `commands.rs` with `#[tauri::command]`
2. **Register** in `lib.rs` → `invoke_handler![]`
3. **Add permission** in `src-tauri/capabilities/default.json`
4. **Frontend wrapper** in `src/lib/tauri.ts`
5. **Use** in React component

---

## 6. Testing

### Rust

```bash
cd src-tauri && cargo test               # all tests
cd src-tauri && cargo test safety        # safety policy
cd src-tauri && cargo test -- --ignored  # expensive/integration tests
```

### Frontend

```bash
npx tsc --noEmit       # type check
npx eslint src/        # lint
```

### Safety Property Tests

```rust
use proptest::prelude::*;
proptest! {
    #[test]
    fn safety_deny_list_never_allows(cmd in "rm -rf /.*") {
        let decision = safety::evaluate(&cmd, &config);
        assert!(matches!(decision, SafetyDecision::Deny { .. }));
    }
}
```

---

## 7. Debugging

```bash
RUST_LOG=debug cargo tauri dev                                    # debug logging
RUST_LOG=aegis_ai_lib::ai::router=trace cargo tauri dev           # trace specific module
```

| Issue | Fix |
|---|---|
| `keyring` error on Linux | Install `gnome-keyring` or `pass` |
| SQLite lock error | Use WAL mode (default) |
| Provider timeout | Check proxy, increase timeout |
| Safety confirmation not showing | Check browser console for errors |

---

## 8. Contributing Workflow

See [CONTRIBUTING.md](../CONTRIBUTING.md) for the full guide.

**Branch naming:** `feat/`, `fix/`, `security/`, `docs/`
**Commit format:** conventional commits (`feat:`, `fix:`, `security:`)

**Security-sensitive changes** (modifications to `safety.rs`, new I/O commands, IPC capability changes, new dependencies, CSP changes) require review by a team member with security expertise.
