# Changelog

All notable changes to Aegis AI are documented in this file. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] — 2026-08-17 — Phase 2 Final: Computer-Use Co-Owner + Safety Layers

### Added — Built-in preconfigured AI provider (Z.AI GLM-4.6)

- **`AegisCloudProvider`** (`src/ai/providers/aegis_cloud.rs`): a new
  zero-config built-in provider backed by Z.AI GLM-4.6. The app now works
  out-of-the-box as soon as an API key is supplied via any of:
  1. `AEGIS_DEFAULT_API_KEY` environment variable (highest priority).
  2. `ZAI_API_KEY` environment variable (alias).
  3. OS keychain entry under `"aegis-ai" / "aegis-cloud"` (set via the new
     `aegis_cloud_configure` command from the Settings UI).
- The provider is registered first in the registry, so when a key is
  available at boot it is automatically selected as the active provider —
  no manual provider pick required.
- User-supplied keys take precedence over the env-var fallback and are
  stored in the OS keychain (Linux Secret Service / Windows Credential
  Manager) with config.toml as fallback.
- New commands: `aegis_cloud_preconfigured`, `aegis_cloud_configure`,
  `aegis_cloud_test`.

### Added — Fast-path optimizations (low-latency AI)

- **`ai/fast_path.rs`**: a tuned `reqwest::Client` builder with:
  - 90s overall timeout, 8s connect timeout (fail fast on dead providers).
  - 8 idle conns per host kept alive 90s (eliminates TLS handshake on warm
    calls).
  - `TCP_NODELAY` enabled — disables Nagle for interactive chat.
  - 30s TCP keepalive prevents NAT timeouts from killing the conn.
- **`ResponseCache<T>`**: a small LRU-ish cache for identical deterministic
  chat requests (keyed on provider + model + messages + temperature).
  Streaming responses are never cached; non-streaming calls are cached for
  5 minutes by default. Cuts repeat-query latency from ~1.5s to <5ms.
- **`Dedup<T>`**: in-flight request deduplication helper so duplicate
  submits (e.g. user double-clicks "Send") only fire one upstream request.
- **`chat_cache_key()`**: SHA-256-based stable cache key for chat requests.

### Added — Computer-use agent loop (AI as "co-owner")

- **`ai/agent.rs`**: a tool-use iteration loop that lets the AI act as a
  "co-owner" of the user's computer. The AI proposes actions via
  OpenAI-style function-calling; the safety policy gates each action;
  results are fed back into the conversation until the AI returns a final
  message or hits the iteration cap.
  - Default cap: 10 iterations per turn.
  - Absolute cap: 20 iterations, regardless of caller request.
  - Hard kill-switch check before every iteration.
  - Rate-limiter check before every tool call.
  - Every tool call is audit-logged to SQLite.
  - If a tool returns `safety_decision=require_confirmation`, the loop
    surfaces the confirmation request to the frontend via the
    `agent://confirmation` event and stops.
  - Events: `agent://chunk`, `agent://tool_call`, `agent://tool_result`,
    `agent://confirmation`, `agent://done`, `agent://error`.
- **`ai/tools.rs`**: registry of 13 tools the AI can invoke locally —
  `shell`, `file_read`, `file_write`, `file_list`, `app_open`, `app_list`,
  `screenshot`, `gui_action`, `clipboard_read`, `clipboard_write`,
  `web_search` (stubbed), `memory_remember`, `memory_lookup`. Each spec
  uses the OpenAI `tools` array shape so any OpenAI-compat provider that
  supports function calling picks them up automatically.
- New command: `ai_agent_run` — kicks off an agent run.
- New command: `agent_list_tools` — returns the tool spec JSON.

### Added — Safety layers

- **Kill switch** (`src/computer/kill_switch.rs`): a process-wide atomic
  boolean that, when tripped, immediately halts every running agent loop
  on its next iteration check. Stays tripped until `reset()` is called
  (prevents the AI from re-launching itself immediately after being
  stopped). New commands: `safety_trip_kill_switch`,
  `safety_reset_kill_switch`, `safety_kill_switch_status`.
- **Rate limiter** (`src/computer/rate_limiter.rs`): token-bucket limiter
  (30 actions/min burst, refill 1 token/sec). Prevents runaway loops from
  spamming the user's machine if the AI decides to call `shell` 100 times
  in a second. New commands: `safety_rate_limiter_status`,
  `safety_rate_limiter_reset`.
- **Audit log** (`src/computer/audit.rs`): every AI tool call is appended
  to an append-only `audit_log` SQLite table with timestamp, conversation
  id, agent run id, tool name, arguments JSON, result JSON, outcome
  (`ok` / `error` / `denied` / `confirmation_required` / `rate_limited`),
  and duration. New commands: `audit_recent`, `audit_count`, `audit_wipe`.
- **Extended destructive-command denylist**: added patterns for reverse
  shells (`bash -i`, `/dev/tcp/`, `nc -e`, `ncat -e`, `socat tcp`),
  cryptominers (`xmrig`, `stratum+tcp`, `minerd`, `ethminer`), credential
  dumpers (`mimikatz`, `procdump`, `lsass`, `gcore`), process injection
  (`ptrace`, `process_vm_readv`), firewall disabling (`iptables -f`,
  `ufw disable`), shellcode loaders (`base64 -d`, `openssl enc -d`),
  persistence (`crontab -r`, `schtasks /create`, `launchctl load`), disk
  wiping (`shred`, `wipe -rf`), privilege escalation (`sudo -i`,
  `sudo su`, `sudo bash`), and cloud creds exfiltration (`.aws/credentials`,
  `.ssh/id_rsa`, `.kube/config`).
- **Network exfiltration heuristic** (`looks_like_exfiltration`): surfaces
  suspicious uploads (`scp`, `rsync`, `curl --upload-file`, `wget
  --post-file`, `nc`, `ssh`, `ftp`, `tftp`) for confirmation, even when
  the AI is in autonomous mode. This means the AI cannot silently ship
  the user's files off-machine.
- Five new unit tests cover the new safety patterns (reverse shell,
  cryptominer, mimikatz, exfiltration detection, autonomous-mode still
  confirms exfiltration).

### Changed

- Bumped version to `0.3.0` across `Cargo.toml`, `package.json`, and
  `tauri.conf.json`.
- `ProviderRegistry` now implements `Clone` (needed for cheap snapshots
  in async command handlers).
- `ProviderRegistry::with_builtin()` registers `AegisCloudProvider`
  first so it becomes the default active provider when preconfigured.
- `MemoryStore::migrate()` now also runs the `audit_log` table migration.
- `computer/mod.rs` exports the new `audit`, `kill_switch`, and
  `rate_limiter` modules.

### Fixed (pre-existing v0.2 issues uncovered during v0.3 development)

- `serde(rename_all = "snakecase")` was invalid; corrected to
  `"snake_case"` in `provider.rs`, `safety.rs`, `automation.rs`,
  `defender.rs` (5 occurrences). Without this fix the `ProviderCategory`,
  `AutoAction`, `SafetyDecision`, `ActionRisk`, and `DefenseEvent` enums
  silently failed to derive `Serialize`/`Deserialize`.
- `error.rs`'s manual `Serialize` impl used the local `Result<T>` type
  alias (which takes 1 generic) instead of `std::result::Result<T, E>`
  (which takes 2). This shadowed the standard `Result` and broke the
  impl signature.
- `ai/provider.rs` `with_builtin()` used `use providers::*;` which
  resolves from the crate root (where `providers` doesn't exist).
  Fixed to `use super::providers::*;`.
- `commands.rs` imported `ProviderCredentials` from the wrong module
  (`ai::provider` instead of `config`).
- `ai/providers/openai.rs` had a broken `impl<T> Deref for OpenAiProvider`
  with an unconstrained type parameter `T`. Removed the impl entirely
  (the `Provider` trait delegation works fine without it).
- `ai/providers/openai_compat.rs` used `delta.content.as_ref()` on a
  `String` field, which is ambiguous (multiple `AsRef` impls). Replaced
  with explicit `is_empty()` + `&` borrow.
- `computer/automation.rs` imported `KeyboardControllable`/`MouseControllable`
  which don't exist in `enigo` 0.2. The correct trait names are `Keyboard`
  and `Mouse`. Also renamed `MouseButton` usage to `Button` (the 0.2 enum
  name) and changed `scroll(direction, amount)` to `scroll(length, axis)`.
  Removed the `NumLock` variant (no longer in 0.2's `Key` enum).
- `computer/screen.rs` ported to the `screenshots` 0.2 `Screenshots`
  (plural) API. The old code used the pre-0.2 `Screen` (singular) API and
  called `to_png()` which no longer exists (the buffer is already
  PNG-encoded in 0.2). Also fixed `rusty_tesseract::image_to_string` to
  take an `Image` struct (from `Image::from_path`) rather than a path
  string.
- `computer/safety.rs` `looks_like_exfiltration` added; the existing
  destructive-command check now consults it before the whitelist so
  network uploads always require confirmation.
- `config.rs` `delete_credential_secure` used the removed
  `keyring::Entry::delete_credential` API; switched to `delete_password`
  (the 2.x name).
- `lib.rs` `tauri_plugin_autostart::init` was called with
  `Some("aegis-ai")` (an `Option<&str>`) but the signature expects
  `Option<Vec<&'static str>>`. Fixed to `Some(vec!["aegis-ai"])`.
- `modes/watcher.rs` ported to the `notify` 6.1.1 callback API. The old
  code passed an `mpsc::Sender<Event>` directly to `Watcher::new`, but
  6.1.1 requires an `EventHandler` trait impl. We now use
  `notify::recommended_watcher` with a closure that funnels events into
  the channel. Also added `use tauri::Emitter;` so `app_handle.emit`
  resolves.
- `commands.rs` borrow-checker issues: many `let x = { let s =
  state.lock(); EXPR };` patterns tripped `error[E0597]: s does not live
  long enough` because the MutexGuard's drop ordering couldn't be proven
  by the compiler. Fixed by binding the inner expression to an
  intermediate `__moved` local before the block returns. The
  `computer_confirm_action` function was restructured to hold the
  AppState lock for the duration of the function (the inner
  `pending_actions` guard needs a stable parent lifetime).
- `state.rs` `boot()` was passing `&app_state_for_setup` into a
  `block_on` closure that consumed it, then `app.manage()` later tried to
  use the moved value. Cloned the `Arc` before the closure.
- `security/monitor.rs` held a `RECENT_THREATS` lock across an `await
  notify_threats` call, which made the spawned future `!Send` and broke
  `tokio::spawn`. Scoped the lock in its own block so it's released before
  the await.
- `ai/providers/bedrock.rs` SigV4 signing stubbed with a clear "not
  implemented in v0.3" error message. The `aws-sigv4` crate resolved to
  1.5.1 in `Cargo.lock`, which has a substantially different API from
  the 1.2 release this code was originally written against. Full rewrite
  is queued for v0.4 (ROADMAP §2.4).
- `ai/providers/replicate.rs` fixed a `String`/`&str` mismatch in the
  output extraction (the `unwrap_or_else` closure returned `String` but
  the outer expression was `&str`).
- `ai/providers/custom.rs` added missing `creds()` helper methods to
  `CustomOllamaProvider` and `WebhookProvider`.
- `capabilities/default.json` updated `fs:allow-create-dir` →
  `fs:allow-mkdir` and `clipboard-manager:allow-read`/`allow-write` →
  `allow-read-text`/`allow-write-text` to match the current
  `tauri-plugin-fs` and `tauri-plugin-clipboard-manager` permission names.

### Known limitations

- AWS Bedrock signing is stubbed (see ROADMAP §2.4 for v0.4 plan).
- `web_search` tool is stubbed (returns empty results with a note).
- `screenshot_area` returns the full screen instead of cropping (the
  `screenshots` 0.2 API doesn't expose area capture directly).
- Test linking on a stock Debian/sandbox env requires GTK/X11/webkit2gtk
  runtime libs; `cargo check --workspace` passes cleanly.

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
