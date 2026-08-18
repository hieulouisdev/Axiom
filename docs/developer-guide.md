# Aegis AI — Developer Guide

**Version:** v0.7 (Phase 4)  
**Date:** 2025-07

---

## 1. Development Setup

### Prerequisites

| Tool | Version | Notes |
|---|---|---|
| Rust | 1.97.1+ | Specified in `rust-toolchain.toml` |
| Node.js | 18+ | LTS recommended |
| Tauri CLI | 2.0 | `cargo install tauri-cli --version "^2.0"` |
| pnpm | 8+ | Preferred package manager |

### Initial Setup

```bash
# Clone the repository
git clone https://github.com/aegis-ai/axiom.git
cd axiom

# Install frontend dependencies
npm install

# Build and run in development mode
cargo tauri dev
```

This starts both the Rust backend (with hot-reload via `cargo-watch`) and
the React frontend (with Vite HMR). The first build compiles all Rust
dependencies, which may take 5–10 minutes.

### Environment Variables

| Variable | Required | Description |
|---|---|---|
| `RUST_LOG` | No | Tracing filter (default: `info,aegis=debug`) |
| `AEGIS_DATA_DIR` | No | Override data directory (for testing) |

### Useful Commands

```bash
# Run Rust tests
cd src-tauri && cargo test

# Run specific test module
cd src-tauri && cargo test safety

# Run frontend type checking
npx tsc --noEmit

# Build production binary
cargo tauri build

# Lint Rust code
cd src-tauri && cargo clippy -- -D warnings

# Check for vulnerable dependencies
cd src-tauri && cargo audit
```

---

## 2. Architecture Overview

```
┌───────────────────────────────────────────────────────────────┐
│                       React Frontend                          │
│  (Chat, Providers, Memory, Security, Modes, Settings)         │
│  State: Zustand store (src/store/index.ts)                    │
│  IPC:   src/lib/tauri.ts → invoke() wrapper                   │
└───────────────────────────┬───────────────────────────────────┘
                            │ invoke<Cmd>("command_name", {…})
                            ▼
┌───────────────────────────────────────────────────────────────┐
│                     Tauri IPC Commands                        │
│              src-tauri/src/commands.rs (50+ commands)         │
└───────────────────────────┬───────────────────────────────────┘
                            │ calls into
                            ▼
┌───────────────────────────────────────────────────────────────┐
│                    AppState (Arc<Mutex<_>>)                    │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐  │
│  │ ConfigStore│ │ AI Router  │ │ MemoryStore│ │ ProviderReg│  │
│  └────────────┘ └────────────┘ └────────────┘ └────────────┘  │
└───────────────────────────┬───────────────────────────────────┘
                            │
        ┌───────────┬───────┼───────┬───────────┐
        ▼           ▼       ▼       ▼           ▼
   ai/ (90+    computer/  security/  memory/    voice/
   providers)  (safety)   (defense)  (SQLite)   (STT/TTS)
```

### Key Subsystems

- **`ai/`** — Multi-provider AI routing. `Provider` trait, `AiRouter`,
  `ProviderRegistry`, provider-specific clients.
- **`computer/`** — Computer-use agent. Safety policy, kill switch, rate
  limiter, audit log, shell exec, file I/O, app launch, GUI automation.
- **`security/`** — Defense subsystem. Process monitor, auto-defender,
  file scanner, quarantine, YARA rules, integrity, network anomaly.
- **`memory/`** — Persistent store. SQLite via `rusqlite`, conversations,
  knowledge base, embeddings (character-trigram hash), RAG.
- **`voice/`** — Voice I/O. STT (cloud Whisper), TTS (OS-native +
  ElevenLabs opt-in), push-to-talk hotkey.
- **`modes/`** — Operational modes. Continuous (60s heartbeat) vs
  on-demand (AI dormant until called).
- **`config.rs`** — Application configuration loaded from `config.toml`.
- **`state.rs`** — Global application state, boot sequence.

---

## 3. How to Add a New AI Provider

All providers implement the `Provider` trait defined in `ai/provider.rs`.
Most providers use the OpenAI-compatible HTTP shape via the
`openai_compat` macro.

### Step-by-step

**1. Create the provider file** at `src-tauri/src/ai/providers/my_provider.rs`:

```rust
use crate::ai::provider::{Provider, ProviderInfo, ChatRequest, ChatResponse, StreamChunk};
use crate::error::Result;

/// MyProvider — a new AI provider.
pub struct MyProvider {
    api_key: String,
    base_url: String,
    model: String,
}

impl MyProvider {
    pub fn new(api_key: &str, base_url: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            model: model.to_string(),
        }
    }
}

// If the provider follows the OpenAI /v1/chat/completions shape,
// use the macro for zero boilerplate:
crate::impl_openai_compat!(MyProvider, "my_provider", "my-provider-api.example.com");

// Otherwise, implement the Provider trait manually:
impl Provider for MyProvider {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: "my_provider".into(),
            name: "My Provider".into(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            supports_streaming: true,
        }
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        // Implement HTTP call to provider API
        todo!("implement chat")
    }

    async fn chat_stream(&self, req: ChatRequest) -> Result<Vec<StreamChunk>> {
        // Implement streaming HTTP call
        todo!("implement streaming")
    }
}
```

**2. Register the module** in `src-tauri/src/ai/providers/mod.rs`:

```rust
pub mod my_provider;
```

**3. Add the provider to the registry** in `ai/provider.rs` or
`ai/providers/mod.rs` where `ProviderRegistry::default()` is populated:

```rust
registry.register("my_provider", ProviderInfo {
    id: "my_provider".into(),
    name: "My Provider".into(),
    base_url: "https://my-provider-api.example.com/v1".into(),
    model: "default-model".into(),
    supports_streaming: true,
});
```

**4. Add to the model catalog** in `ai/catalog.rs` if the provider has
multiple models.

**5. Test the provider:**

```bash
cd src-tauri && cargo test my_provider
```

**6. Update the provider list** in `docs/PROVIDERS.md` and the frontend
provider selector component.

---

## 4. How to Add a New Agent Tool

Agent tools are defined in `ai/tools.rs` as variants of the `AgentTool`
enum and dispatched in the agent loop.

### Step-by-step

**1. Define the tool** in `src-tauri/src/ai/tools.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "tool", rename_all = "snake_case")]
pub enum AgentTool {
    // ... existing tools ...
    
    /// My new tool — does something useful.
    MyNewTool {
        /// Description of the parameter.
        param: String,
    },
}
```

**2. Implement the tool handler** in the agent dispatch function
(`ai/agent.rs` or wherever tools are dispatched):

```rust
AgentTool::MyNewTool { param } => {
    // 1. Evaluate safety — all tools must go through the safety policy
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
    
    // 2. Execute the tool logic
    let result = do_my_tool_work(&param).await?;
    
    // 3. Log to audit
    audit::log("agent.my_new_tool", &format!("param={param}"), None);
    
    // 4. Return result
    Ok(result)
}
```

**3. Add the tool to `agent_list_tools`** command response so the AI
knows it's available.

**4. Write tests:**

```rust
#[tokio::test]
async fn my_new_tool_works() {
    let result = do_my_tool_work("test").await.unwrap();
    assert_eq!(result, "expected");
}
```

---

## 5. How to Add a New Tauri Command

All Tauri commands are defined in `src-tauri/src/commands.rs` and
registered in `lib.rs`.

### Step-by-step

**1. Define the command** in `commands.rs`:

```rust
/// My new command — does something useful.
#[tauri::command]
pub async fn my_new_command(
    state: tauri::State<'_, SharedState>,
    param: String,
) -> Result<String, String> {
    let mut app = state.lock();
    app.my_subsystem
        .do_something(&param)
        .map_err(|e| e.to_string())
}
```

**2. Register the command** in `lib.rs`:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    commands::my_new_command,
])
```

**3. Add to the capability file** `src-tauri/capabilities/default.json`:

```json
{
  "identifier": "default",
  "windows": ["main"],
  "permissions": [
    "core:default",
    // ... existing permissions ...
    "allow-my-new-command"
  ]
}
```

**4. Add the frontend wrapper** in `src/lib/tauri.ts`:

```typescript
export async function myNewCommand(param: string): Promise<string> {
  return invoke<string>("my_new_command", { param });
}
```

**5. Use in a React component:**

```typescript
import { myNewCommand } from "../lib/tauri";

const result = await myNewCommand("hello");
```

---

## 6. Testing Guidelines

### Rust Tests

```bash
# Run all tests
cd src-tauri && cargo test

# Run a specific module
cd src-tauri && cargo test safety
cd src-tauri && cargo test embeddings
cd src-tauri && cargo test kill_switch

# Run with output
cd src-tauri && cargo test -- --nocapture

# Run ignored tests (expensive/integration)
cd src-tauri && cargo test -- --ignored
```

### Frontend Tests

```bash
# Type checking
npx tsc --noEmit

# Lint
npx eslint src/
```

### Integration Tests

The best way to test end-to-end is with `cargo tauri dev`:

1. Start the dev server.
2. Configure a real or mock AI provider.
3. Test the full chat → safety → audit flow.

### Safety Policy Tests

The safety policy has comprehensive unit tests that verify:

- Whitelisted commands → `Allow`
- Dangerous commands → `RequireConfirmation` or `Deny`
- System path writes → `Deny`
- Bypass mode respects hard-deny list

Run them with: `cd src-tauri && cargo test safety`

### Property-Based Testing

For security-critical code, use `proptest` to verify invariants:

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

## 7. Debugging Tips

### Rust Backend

```bash
# Enable debug logging
RUST_LOG=debug cargo tauri dev

# Trace a specific module
RUST_LOG=aegis_ai_lib::ai::router=trace cargo tauri dev

# Attach a debugger (lldb)
lldb --attach-pid $(pgrep aegis-ai)
```

### Frontend

- Open DevTools with `Ctrl+Shift+I` (development builds only).
- Check the browser console for IPC errors.
- Use React DevTools to inspect component state.

### Common Issues

| Issue | Cause | Fix |
|---|---|---|
| `keyring` error on Linux | No Secret Service daemon | Install `gnome-keyring` or `pass` |
| SQLite lock error | Multiple connections | Use WAL mode (default) |
| Provider timeout | Network/firewall | Check proxy settings, increase timeout |
| Safety confirmation not showing | Frontend error handler | Check browser console for errors |

### Logging

The Rust backend uses `tracing` with structured logging. Key events:

- `aegis=debug` — General application flow.
- `aegis_ai_lib::ai=trace` — AI provider requests/responses.
- `aegis_ai_lib::security=debug` — Security monitor events.
- `aegis_ai_lib::computer=debug` — Computer-use agent actions.

---

## 8. Contributing Workflow

### Branch Naming

- `feat/<description>` — New features.
- `fix/<description>` — Bug fixes.
- `security/<description>` — Security-related changes.
- `docs/<description>` — Documentation updates.

### Pull Request Process

1. **Fork** the repository and create a feature branch.
2. **Implement** your change with tests.
3. **Run checks:**
   ```bash
   cd src-tauri && cargo clippy -- -D warnings
   cd src-tauri && cargo test
   cd src-tauri && cargo audit
   npx tsc --noEmit
   ```
4. **Document** any new commands, providers, or tools.
5. **Submit** a PR with a clear description of the change and its security
   implications.

### Security-Sensitive Changes

Changes that affect the security subsystem require additional review:

- Modifications to `computer/safety.rs` (safety policy).
- New Tauri commands that perform I/O.
- Changes to the IPC capability file.
- New dependencies (supply chain risk).
- Changes to the CSP configuration.

These changes should be reviewed by at least one team member with security
expertise.

### Code Style

- **Rust:** Follow `rustfmt` defaults + `clippy` pedantic warnings.
- **TypeScript:** Follow the existing project conventions (Prettier + ESLint).
- **Commits:** Use conventional commit format (`feat:`, `fix:`, `security:`).
