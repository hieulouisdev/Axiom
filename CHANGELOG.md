# Changelog

All notable changes to Aegis AI are documented in this file. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-08-17 — Phase 1 Foundation Skeleton

### Added

- **Project scaffolding**: Tauri 2.0 + Rust 1.97.1 + React 18 + TypeScript +
  Tailwind CSS. White-themed UI with sidebar navigation.
- **AI provider trait + registry** with 33 providers:
  - Cloud major (10): OpenAI, Anthropic Claude, Google Gemini, DeepSeek,
    Groq, OpenRouter, Mistral, Cohere, Together AI, Anyscale.
  - Local (9): Ollama, LM Studio, LocalAI, llama.cpp, GPT4All, Jan,
    KoboldCpp, vLLM, Llamafile.
  - Cloud other (9): Azure OpenAI (stub), AWS Bedrock (stub), HuggingFace,
    Replicate (stub), Moonshot, Zhipu, Yi, DeepInfra, Fireworks.
  - Custom (4): Custom OpenAI-compat, Custom Anthropic-compat, Custom
    Ollama-compat, generic webhook.
- **OpenAI-compatible shared client** (`openai_compat.rs`) handles chat and
  streaming for every OpenAI-compat provider.
- **Anthropic Messages API** client with system-prompt separation.
- **Google Gemini** `generateContent` client.
- **Ollama native API** client with `ping` via `/api/tags`.
- **AI router**: active-provider resolution, credential injection,
  cost-saving active-default fallback.
- **Computer-use subsystem**:
  - 5-level safety policy (`Safe` → `Critical`) with whitelist + denylist.
  - Shell exec, file read/write (with 1 MB cap), app launch, GUI
    automation stubs, screenshot stub.
  - Hard-deny for system paths and destructive commands.
- **Security subsystem**:
  - Process monitor: 15s poll of `/proc` (Linux) or ToolHelp snapshot (Windows).
  - Threat signature matching with regex (substring for v0.1).
  - Auto-defense: notify → quarantine → kill escalation by severity.
  - File quarantine store with restore / delete.
  - On-demand virus scanner: SHA-256 hash match against sample signatures
    (EICAR test file).
- **Memory store**: SQLite with `conversations`, `messages`, `activities`,
  `knowledge`, `events` tables. Migrations run on boot.
- **Operational modes**: Continuous (60s heartbeat) and On-demand (AI
  dormant, security monitor still runs).
- **i18n**: English (default) + Vietnamese, ~80 keys, runtime switch.
- **Config persistence**: `config.toml` in user data dir.
- **25+ Tauri IPC commands** bridging Rust ↔ TypeScript.
- **Documentation**: README, ROADMAP (4-phase plan), PRIVACY, SECURITY,
  CONTRIBUTING, ARCHITECTURE, PROVIDERS, SAFETY.
- **GitHub Actions workflow** for Linux + Windows builds.
- **MIT license**.

### Known limitations

- GUI automation calls (`mouse_move`, `mouse_click`, `type_text`,
  `press_key`) are no-ops in v0.1; Phase 2 adds `enigo` integration.
- Screenshot returns a 1x1 placeholder PNG; Phase 2 adds real capture.
- Token-based safety confirmation is a stub; the frontend is expected to
  re-issue the original request with `authorized=true` after the user
  confirms.
- Streaming chat is wired but `ai_chat_stream` returns an error; Phase 2
  exposes it via Tauri events.
- Azure OpenAI, AWS Bedrock, and Replicate are stubs (need bespoke auth).
- API keys are stored in plaintext in `config.toml`; Phase 2 moves them to
  the OS keychain.
- The QuarantineStore lives inside the defender task; Phase 2 lifts it to
  `AppState` so the UI can read it.
