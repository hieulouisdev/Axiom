# Changelog

All notable changes to Aegis AI are documented in this file. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] — 2026-08-17 — Phase 2 Production Features

### Added

- **Streaming chat over Tauri events**:
  - `ai_chat_stream` command emits `chat://chunk` events with delta text.
  - `ai_chat_cancel` command cancels streaming via `CancellationToken`.
  - Frontend renders incremental deltas with "Stop generation" button.
  - Events: `chat://chunk`, `chat://done`, `chat://error`, `chat://cancelled`.
- **Full computer-use agent with `enigo`**:
  - Cross-platform mouse/keyboard simulation via `enigo` crate.
  - `mouse_move`, `mouse_click`, `type_text`, `press_key`, `mouse_scroll`.
  - Key combo parsing: Ctrl+C, Alt+Tab, Enter, F1-F12, arrows, etc.
  - `AutoAction::MouseScroll` for wheel scrolling.
- **Real screen capture + OCR**:
  - `screenshots` crate captures the primary display as PNG.
  - `screenshot_area()` for partial screen capture.
  - `rusty-tesseract` for OCR text extraction from screenshots.
- **Token-based action confirmation**:
  - `computer_request_action` creates a pending token with 60-second TTL.
  - `computer_confirm_action` validates the token and authorizes the action.
  - `PendingAction` stored in `AppState.pending_actions`.
- **Real cloud provider implementations**:
  - Azure OpenAI: `api-key` header auth, deployment-id URL routing, `api-version` param.
  - AWS Bedrock: SigV4 request signing via `aws-sigv4` crate, Anthropic Messages format.
  - Replicate: async Prediction API (POST → poll → result).
  - HuggingFace: now using real OpenAI-compat endpoint (was stub).
- **OS keychain credential storage**:
  - `keyring` crate stores API keys in OS keychain (Linux Secret Service / Windows Credential Manager).
  - Falls back to config.toml if keyring is unavailable.
  - `ai_configure_provider` automatically uses keyring for API keys.
- **Conversation summarization**:
  - `memory_summarize` command uses the active AI provider to summarize conversations.
  - Keeps summaries under 200 words for efficient context management.
- **Better security signals**:
  - Real network anomaly detection using `procfs` (Linux) / `netstat` (Windows).
  - Detects suspicious listeners and outbound connections on known malware ports.
  - File integrity monitor: SHA-256 baseline of critical files (`.bashrc`, `authorized_keys`, autostart, etc.).
  - `security_integrity_check` and `security_integrity_save_baseline` commands.
  - `security_network_scan` command for on-demand network scanning.
  - Baselines persisted to SQLite across restarts.
- **Webhook/email security alerts**:
  - `send_alert` POSTs defense events to configured webhook URL.
  - Slack/Discord/custom endpoint support.
  - Integrated into auto-defense event loop.
- **File system watcher**:
  - `notify` crate watches configured directories for changes.
  - Emits `watcher://change` events to frontend for proactive AI analysis.
  - Configurable ignore patterns (*.tmp, .git/*, node_modules/*, etc.).
- **Clipboard monitoring and control**:
  - `clipboard_read_cmd`, `clipboard_write_cmd` for read/write.
  - `clipboard_watch_start_cmd`, `clipboard_watch_stop_cmd` for change detection.
  - Platform-specific: xclip/xsel/wl-paste (Linux), PowerShell (Windows).
- **System tray integration**:
  - Tray icon with Show/Hide/Quit menu.
  - Click to show/focus the main window.
- **Additional Tauri plugins**:
  - `tauri-plugin-autostart`: launch on system startup.
  - `tauri-plugin-global-shortcut`: global hotkeys for push-to-talk, quick summon.
  - `tauri-plugin-process`: process management.
- **Quarantine store lifted to AppState**:
  - `security_quarantine_list` now returns real quarantine entries.
  - `security_restore_file` now works properly.
- **ClamAV integration** (already in scanner.rs):
  - Delegates to `clamdscan` when available on PATH.
  - Falls back to hash-based scanning when ClamAV is absent.
  - Windows Defender integration via `MpCmdRun`.
  - Loads ClamAV-style .h9db/.hsb hash signature files.
- **Custom app logo**:
  - All icons (32x32, 128x128, 256x256, icon.ico) generated from user's logo.
  - Logo displayed in sidebar and system tray.
- **Version bump to 0.2.0** across all config files.

### Changed

- `ai_chat_stream` is no longer a stub — fully implemented with Tauri events.
- `computer_automate` now uses real `enigo` instead of no-op stubs.
- `computer_screenshot` now captures the real screen instead of a placeholder.
- `computer_confirm_action` validates tokens instead of returning an error.
- `security_quarantine_list` returns real entries from `AppState.quarantine`.
- `security_restore_file` calls `QuarantineStore::restore()`.
- `security_status` now includes `network_anomalies` field.
- Azure OpenAI, Bedrock, Replicate are now fully implemented providers.
- API keys are stored in OS keychain when available (fallback to config.toml).

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
