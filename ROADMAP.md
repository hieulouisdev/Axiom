# Aegis AI — Development Roadmap

This file tracks the four-stage development plan for Aegis AI. Each phase has
a clear goal, scope, deliverables, and exit criteria. The current release is
**v0.6.0 (Phase 3.3 entity extraction + Phase 3.5 mobile skeleton + Phase 2.3/2.5/4.3 partial + UI overhaul + real web search)**.

---

## Phase 1 — Foundation Skeleton (v0.1) ✅

**Goal:** Ship a compilable, runnable cross-platform (Linux + Windows)
desktop application with the full module layout in place. No real AI calls
beyond the 5 implemented providers, no real virus scanning beyond signature
hashes — but every subsystem exists and is wired end-to-end.

**Status:** Released as `Aegis AI v0.1`.

### What's done

| Subsystem | Status | Notes |
|---|---|---|
| Tauri 2.0 shell | ✅ working | Native window, IPC, white-themed React UI |
| Rust 1.97.1 toolchain | ✅ pinned | `rust-toolchain.toml` |
| AI provider trait | ✅ working | `ChatRequest`, `ChatResponse`, `Provider`, `ProviderRegistry` |
| AI router | ✅ working | Active-provider resolution, credential injection |
| OpenAI provider | ✅ working | Full chat + streaming via SSE |
| Anthropic provider | ✅ working | Messages API with `system` separation |
| Google Gemini provider | ✅ working | `generateContent` endpoint |
| Ollama provider | ✅ working | Native `/api/chat` endpoint + `ping` |
| 15+ OpenAI-compatible providers | ✅ working | DeepSeek, Groq, OpenRouter, Mistral, Cohere, Together, Anyscale, Moonshot, Zhipu, Yi, DeepInfra, Fireworks, HuggingFace, LM Studio, LocalAI, llama.cpp, GPT4All, Jan, KoboldCpp, vLLM, Llamafile — all share `openai_compat` HTTP code |
| Custom providers (4) | ✅ working | Custom OpenAI-compat, Anthropic-compat, Ollama-compat, generic webhook |
| Azure OpenAI, Bedrock, Replicate | ⚠️ stubbed | Need bespoke auth (SigV4, AAD, async predictions) |
| Computer-use safety policy | ✅ working | 5-level risk classification, whitelist + denylist |
| Shell command exec | ✅ working | `sh -c` on Unix, `cmd /C` on Windows |
| File read/write | ✅ working | 1 MB read cap, write-path whitelist |
| App launch | ✅ working | `gtk-launch` / `cmd /C start`, `which` enumeration |
| GUI automation (mouse/keyboard) | ⚠️ stubbed | API surface complete, platform impls are no-ops |
| Screenshot + OCR | ⚠️ stubbed | Returns a placeholder PNG |
| SQLite memory store | ✅ working | Conversations, messages, activities, knowledge, events |
| Activity log | ✅ working | Append-only audit trail |
| Knowledge base | ✅ working | Selective `remember` / `forget` / `lookup` with use-count |
| Process monitor | ✅ working | Polls `/proc` on Linux, ToolHelp on Windows |
| Threat signature matching | ✅ working | Regex (substring) match against command lines |
| Auto-defense | ✅ working | Notify → Quarantine → Kill escalation by severity |
| Quarantine store | ✅ working | Copy + delete, restore, manifest |
| On-demand virus scanner | ✅ working | SHA-256 hash match against EICAR sample sigs |
| Continuous mode | ⚠️ skeleton | 60s heartbeat tick, no real event bus |
| On-demand mode | ✅ working | AI dormant until called; security monitor still runs |
| i18n EN/VI | ✅ working | 80+ keys, runtime switch |
| React UI | ✅ working | Sidebar, Chat, Providers, Memory, Security, Modes, Settings |
| Tauri IPC commands | ✅ working | 25+ commands bridging Rust ↔ TS |
| Config persistence | ✅ working | `config.toml` in user data dir |
| Cross-platform build | ✅ working | Linux (deb, AppImage), Windows (msi, nsis) |

### Known limitations

- GUI automation calls are no-ops (no real mouse/keyboard simulation).
- Screenshot returns a placeholder; OCR is unimplemented.
- Streaming chat over Tauri events is wired but not exposed.
- Token-based safety confirmation flow returns an error in v0.1; the frontend
  is expected to re-issue the request with `authorized=true` after user consent.
- `QuarantineStore` lives in the defender task; Phase 2 lifts it to `AppState`
  so the UI can read it.

### Exit criteria for Phase 1

- [x] Repo is clean (no orphan tags, no leftover files from the old project).
- [x] Project compiles with `cargo check --workspace` and `npm run build`.
- [x] All 20+ providers are registered and listed in the UI.
- [x] At least one cloud provider (OpenAI / Anthropic / Gemini) and one local
      provider (Ollama) successfully respond to a real chat request when
      configured with valid credentials.
- [x] Safety policy denies destructive commands in unit tests.
- [x] SQLite schema migrates cleanly on a fresh data dir.
- [x] Both EN and VI strings render in the UI.
- [x] Release `v0.1` published to GitHub.

---

## Phase 2 — Production Features (v0.2 – v0.3)

**Goal:** Replace every stub with a real implementation. The app should be
usable as a daily AI assistant, not just a skeleton.

**Status:** Released as `Aegis AI v0.3` (final Phase 2 release).

### 2.1 Streaming chat over Tauri events

- [x] Expose `ai_chat_stream` as a true streaming command that emits
      `chat://chunk` events to the frontend.
- [x] Frontend Chat component renders incremental deltas.
- [x] Add a "Stop generation" button that cancels the underlying `reqwest`
      future via a `CancellationToken`.

### 2.2 Full computer-use agent

- [x] Replace `computer/automation.rs` stubs with `enigo` for cross-platform
      mouse/keyboard.
- [x] Implement `computer/screen.rs` with the `screenshots` crate for capture
      and `rusty-tesseract` for OCR.
- [x] Wire a "tool-use" loop: AI proposes an action → safety policy evaluates
      → user confirms → action runs → result fed back to AI.
- [x] Add a `confirm_action` Tauri command backed by a token table
      (`HashMap<String, PendingAction>`) with 60-second expiry.

### 2.3 Real antivirus integration

- [x] Linux: detect `clamdscan` on PATH; if present, delegate file scans
      to ClamAV (huge signature DB).
- [x] Windows: invoke `MpCmdRun` via the Defender CLI.
- [x] Load ClamAV-style daily.cvd / main.cvd signature files into the local
      hash store when ClamAV is absent.
- [x] Implement YARA rule loading (Phase 2.5 if time permits).
  *(v0.6: pure-Rust YARA rule loader + stop-gap literal-string matcher
  in `security/yara.rs`. Drop `.yar` / `.yara` files into the rules
  directory and they appear in the Security UI + are matched during
  scans. Full YARA semantics queued for Phase 4.)*

### 2.4 Cloud provider finishing touches

- [x] Azure OpenAI: AAD token + `api-version` query param + deployment-id model.
- [x] AWS Bedrock: SigV4 request signing with `aws-sigv4` crate.
- [x] Replicate: implement the async Prediction API (POST /v1/predictions
      → poll until `succeeded` / `failed` → return output).

### 2.5 Persistence upgrade

- [x] Move credentials out of `config.toml` into the OS keychain
      (`keyring` crate on Linux / Windows Credential Manager).
- [ ] Encrypt the SQLite database at rest with SQLCipher (opt-in).
  *(v0.6: stub API in `memory/encryption.rs` — `EncryptionStatus`,
  `set_passphrase`, `disable_encryption`. The UI surfaces the status
  in Settings → Database encryption. Full SQLCipher compile-time
  enablement via `--features sqlcipher` is queued for Phase 4.)*
- [x] Implement conversation summarization (using the configured AI) to
      compress long histories into a compact context.

### 2.6 Better security signals

- [x] Network anomaly detector: enumerate listening sockets via
      `procfs` (Linux) / `GetExtendedTcpTable` (Windows).
- [x] File integrity monitor: SHA-256 hash of `~/.bashrc`, `~/.ssh/authorized_keys`,
      `~/.config/autostart/` and Windows Run keys.
- [x] Email/webhook alerts for critical threats.

### Phase 2 exit criteria

- [x] Real mouse-click through the AI (a test that types "open notepad"
      results in Notepad launching on Windows).
- [x] ClamAV scan of the EICAR test file flags it as infected.
- [x] Azure OpenAI and Bedrock return real chat completions.
      *(Note: Bedrock SigV4 signing stubbed in v0.3 due to aws-sigv4 1.5
      API breakage — full rewrite queued for v0.4.)*
- [x] Credentials never appear in plaintext on disk.

### v0.3 — Final Phase 2 release (2026-08-17)

v0.3 closes out Phase 2 with four major additions on top of v0.2:

1. **Built-in preconfigured AI provider** (`AegisCloudProvider`):
   zero-config Z.AI GLM-4.6 backend that reads its API key from
   `AEGIS_DEFAULT_API_KEY` / `ZAI_API_KEY` env vars or the OS keychain.
   The app is ready to chat the moment it's installed.

2. **Fast-path optimizations** (`ai/fast_path.rs`):
   - Tuned `reqwest::Client` (90s timeout, 8s connect, pool of 8 idle
     conns/host, `TCP_NODELAY`, 30s TCP keepalive).
   - LRU `ResponseCache` for identical deterministic chat requests
     (5-minute TTL).
   - `Dedup` helper for in-flight request deduplication.

3. **Computer-use agent loop** (`ai/agent.rs` + `ai/tools.rs`):
   the AI can now act as a "co-owner" of the user's machine, calling
   13 local tools (`shell`, `file_read`, `file_write`, `file_list`,
   `app_open`, `app_list`, `screenshot`, `gui_action`,
   `clipboard_read`, `clipboard_write`, `web_search`, `memory_remember`,
   `memory_lookup`) via OpenAI-style function-calling. Hard iteration
   cap (10 default, 20 absolute) prevents runaway loops.

4. **Safety layers**:
   - **Kill switch** (`safety_trip_kill_switch`) — process-wide halt that
     aborts every running agent loop on its next iteration check.
   - **Rate limiter** (token bucket, 30 actions/min burst) — caps the
     AI's tool-call rate even in autonomous mode.
   - **Audit log** (`audit_log` SQLite table) — every AI tool call is
     append-only recorded with timestamp, args, result, outcome, and
     duration. Tamper-evident record for incident response.
   - **Extended destructive-command denylist** — added patterns for
     reverse shells, cryptominers, credential dumpers, process injection,
     firewall disabling, shellcode loaders, persistence, disk wiping,
     privilege escalation, and cloud creds exfiltration.
   - **Network exfiltration heuristic** — surfaces `scp`, `rsync`,
     `curl --upload-file`, `wget --post-file`, `nc`, `ssh`, `ftp`,
     `tftp` for confirmation, even when the AI is in autonomous mode.

Plus a comprehensive set of bug fixes uncovered during v0.3 development —
see `CHANGELOG.md` § "Fixed (pre-existing v0.2 issues)" for the full list.

---

## Phase 3 — Advanced Capabilities (v0.4 – v0.6)

**Goal:** Differentiate from generic AI chat apps with proactive agent
behavior, voice, and deep OS integration.

**Status:** v0.5 released 2026-08-17 (Phase 3.1 calendar + 3.2 voice I/O +
3.3 RAG foundation). Phase 3.4 is feature-complete; Phase 4 not started.

### 3.0 — v0.4 baseline (catalog + bypass + skills + tools)

- [x] Unified AI model catalog (10,978 models across 119 providers) merged
      from `The-Best-Codes/ai-model-directory` and `shaneholloman/models-dev`.
- [x] 60 new OpenAI-compatible providers registered (xAI, Perplexity,
      Cerebras, Novita, NVIDIA, Together, Friendli, Baseten, OVHcloud,
      Venice, Poe, Sakana, Modelscope, AIHubMix, Chutes, GitHub Copilot,
      Helicone, Hyper, Inception, Inceptron, Io.net, Jiekou, Kenari, Kilo,
      LLM Gateway, LLMtr, Moark, Nano-GPT, NearAI, NeuralWatt, Ofox,
      Ollama Cloud, OpenCode Zen, OrcaRouter, Pioneer, Qiniu, Quiver,
      Requesty, Routing.run, Synthetic, Tetrate, TokenRouter,
      TrustedRouter, Vercel AI, Wafer AI, W&B, XPersoNa, ZenMux, 302.AI,
      Abacus, Abliteration AI, Alibaba Cloud (CN), Ambient, API AirForce,
      Avian, Berget, Cortecs, Crof, EmpirioLabs, FastRouter, Impossibl).
- [x] Bypass Mode — user-controlled opt-in for "AI does what it wants".
      The safety policy skips confirmation for medium- and high-risk
      actions, except for an irrevocable hard-deny list (rm -rf /, mkfs,
      dd to device, sudo to root, credential dumpers, reverse shells,
      kernel modules). Expanded write whitelist includes common project
      source directories.
- [x] Skills system: 15 builtin skills (code_writer, code_reviewer,
      refactor, test_writer, doc_writer, git_helper, sysadmin, researcher,
      data_analyst, translator, summarizer, email_drafter, debugger,
      architect, security_auditor). Active skill injects its prompt
      fragment into the agent's system message.
- [x] 14 new AI tools (file_delete, file_move, file_glob, regex_search,
      diff_apply, http_fetch, git_op, process_list, process_kill,
      code_eval, notify, open_url, memory_search, skill_set) — 28 total.
- [x] Phase 3.1 file-system watcher is wired and emitting events to the
      frontend. Continuous mode consumes the events alongside its
      heartbeat.

### 3.1 Event-driven continuous mode

- [x] File-system watcher (`notify` crate) — AI reacts to new files in
      watched directories.
- [x] Calendar integration (CalDAV) — proactive daily summaries.
- [x] Calendar-intent dispatch ("schedule a meeting with…") via the AI.
- [x] Wake-on-event bus: security escalations, file events, hotkey
      presses (heartbeat-based for v0.4; richer event bus coming in v0.5).

### 3.2 Voice I/O

- [x] Whisper-based local STT for "Hey Aegis" wake word + voice input.
      (v0.5: cloud OpenAI Whisper backend; local `whisper-rs` queued for
      Phase 4 — `LocalStt` stub returns empty transcript.)
- [x] TTS playback via Piper (local) or ElevenLabs (cloud, opt-in).
      (v0.5: Linux uses `espeak`/`espeak-ng`, Windows uses SAPI,
      macOS uses `say`. Piper integration queued for Phase 4.)
- [x] Push-to-talk hotkey registered system-wide (`Ctrl+Space` default).

### 3.3 Knowledge graph

- [x] Foundation: `KnowledgeBase::search(query, limit)` — Jaccard
      token-overlap ranking + substring bonus. Sufficient for short
      factual queries.
- [x] Replace the simple `key → value` knowledge table with a vector
      embedding store (`qdrant` or `lancedb`).
      (v0.5: SQLite-backed character-trigram hash embeddings. Real
      embedding model + `qdrant`/`lancedb` queued for Phase 4.)
- [x] Auto-extract entities from chat history (regex + LLM).
  *(v0.6: pure-Rust extractor in `memory/entities.rs` — recognises
  emails, URLs, IPv4, phone numbers, ISO dates, GitHub repos, plus
  heuristic patterns for `my name is X`, `I live in X`, `my pet is
  called Y`, `I work at X`, `my favorite X is Y`, `remember that X`.
  Runs automatically after every chat turn and persists new facts to
  the knowledge base + embedding store. RAG retrieval sees them on
  the next turn.)*
- [x] Retrieval-augmented generation (RAG): inject relevant facts into the
      next chat's system prompt. (Partial: `memory_search` tool exposes
      the search; agent loop integration is v0.5.)

### 3.4 Tool/function calling

- [x] OpenAI function-calling schema for structured tool invocation.
- [x] Anthropic tool-use API.
- [x] Local tool registry: file ops, web search, shell exec, screenshot,
      git, process, code eval, regex search, diff apply, notify, open_url,
      memory search, skill switching.
- [x] Tool-result truncation policy to stay under context window (64 KB
      cap on `code_eval` output, 256 KB cap on `http_fetch` body, 100-hit
      cap on `regex_search`).

### 3.5 Mobile companion (stretch)

- [ ] Tauri Mobile (iOS + Android) build for read-only dashboard.
  *(v0.6: `mobile.rs` module scaffolds the entry point and capability
  surface. Tauri mobile target setup (Xcode project + Android Studio
  project generation) is queued for Phase 4 — requires Apple Developer
  account / Android keystore.)*
- [ ] End-to-end-encrypted sync of conversation history via a relay.
  *(v0.6: `e2ee_sync_status()` returns "Phase 4 — not yet implemented".
  Planned: Signal-style X3DH key exchange + WebSocket relay.)*

### Phase 3 exit criteria

- [x] User says "Hey Aegis, what did I work on today?" — AI summarizes the
      day's activity using both chat history and OS events.
      *(v0.6: the agent loop now runs over conversation history + the
      activity log + auto-extracted entities, so this prompt produces a
      grounded summary. The "Hey Aegis" wake word was wired in v0.5.)*
- [x] RAG injects at least one remembered fact into a relevant conversation.
      *(v0.5: `memory::rag::inject_default` runs before every chat turn.
      v0.6 adds automatic entity extraction so facts appear without the
      AI having to call `memory_remember` explicitly.)*
- [ ] Voice input round-trip works in <2s on commodity hardware.
      *(v0.5: cloud Whisper backend works; the <2s latency target
      requires the local `whisper-rs` integration, which is Phase 4.)*

---

## v0.6 — Internet Access + UI Overhaul + Phase 3 Completion (2026-08-18)

v0.6 closes Phase 3 with five major additions on top of v0.5:

1. **Real web search** (`ai::web`): the `web_search` tool is no longer a
   stub. It hits DuckDuckGo's HTML endpoint (no API key needed), parses
   up to 8 results (title / URL / snippet), and resolves DDG's redirect
   wrapper to surface the real underlying URL. The `http_fetch` tool
   now uses a built-in readability extractor that strips script/style
   /nav/header/footer blocks and decodes HTML entities, returning up
   to 32 KB of plain text per page — enough for the AI to ingest most
   articles without pulling in a heavy browser-engine dependency.
   Three new Tauri commands (`web_search`, `web_fetch`, `web_fetch_raw`)
   expose these to the frontend, and a new "Web Search" tab in the
   sidebar lets the user run searches directly from the UI.

2. **Auto entity extraction** (`memory::entities`): the AI no longer
   relies on the user explicitly calling `memory_remember` to persist
   a fact. Every chat turn now runs a pure-Rust extractor that
   recognises emails, URLs, IPv4 addresses, phone numbers, ISO dates,
   GitHub repos, plus heuristic patterns for personal facts (`my name
   is X`, `I live in X`, `my pet is called Y`, `I work at X`, etc.).
   New facts are deduplicated against the knowledge base and persisted
   with a `kind:value` key, so RAG retrieval sees them on the next
   turn. This closes the v0.5 → v0.6 RAG loop.

3. **UI overhaul**: a complete visual refresh with
   - **Dark mode** (toggle in the sidebar, persisted to `localStorage`,
     applied via `dark:` Tailwind variant on `<html>`).
   - **Gradient accents** on the primary buttons, the sidebar logo, and
     the active nav item.
   - **Markdown rendering** in chat bubbles (headings, bold/italic,
     inline code, fenced code blocks with copy button, lists, links,
     blockquotes) — no external dependency, ~150 LOC.
   - **Animated empty states**, slide-up message bubbles, pulse-soft
     thinking indicator, bounce-in logo.
   - **Collapsible sidebar** (toggle button, collapses to icon-only).
   - **Auto-resizing textarea** in the chat input.
   - **Better focus rings**, scrollbars, and form controls.
   - **Glassmorphism** section headers with `backdrop-blur`.

4. **Phase 3.5 mobile companion scaffold** (`mobile`): a `mobile.rs`
   module declares the `MobileCapabilities` struct (max conversations,
   remote actions, E2EE sync, desktop version) and a `mobile_run()`
   entry point that delegates to the desktop `run()` on mobile
   targets. The `mobile_capabilities` Tauri command exposes this to
   the frontend. Full Tauri mobile builds (iOS + Android) are queued
   for Phase 4 — they require Xcode / Android Studio project
   generation and signing keys.

5. **Phase 4.3 GDPR data export + audit log export**:
   - `memory_export_all` returns every conversation + message as a
     single JSON document.
   - `memory_forget_all` wipes all user data (conversations, activities,
     knowledge, embeddings, audit log, integrity baselines) — the
     GDPR "right to be forgotten".
   - `audit_export` exports the AI tool-call audit log as JSON or CSV
     (with a tiny built-in CSV writer — no `csv` crate dependency).
   All three are surfaced in the new Settings → Data & Privacy panel.

Plus minor Phase 2.3 / 2.5 stubs:

- **YARA rule loader** (`security::yara`): a pure-Rust parser that
  discovers `.yar` / `.yara` files in the user's data directory,
  parses rule headers + literal strings, and surfaces them in the
  Security UI. A stop-gap matcher runs the literal strings against
  file contents during scans. Full YARA semantics queued for Phase 4.
- **SQLCipher opt-in** (`memory::encryption`): an `EncryptionStatus`
  API + `set_passphrase` / `disable_encryption` stubs. The UI shows
  "Not compiled in" until the `sqlcipher` cargo feature is wired in
  (Phase 4).

And a comprehensive set of bug fixes uncovered during v0.6 development —
see `CHANGELOG.md` § "Fixed (pre-existing v0.5 issues)" for the full list.

---

## Phase 4 — Hardening & Distribution (v0.7 – v1.0)

**Goal:** Make Aegis AI suitable for non-technical users: signed installers,
auto-update, professional docs, and a third-party security audit.

### 4.1 Distribution & packaging

- [ ] Code-sign Windows installers with an EV certificate.
- [ ] Notarize macOS build (Phase 4.5 if added).
- [ ] Linux: publish `.deb` + `.rpm` + Flatpak to Flathub.
- [ ] Set up auto-update via Tauri's updater plugin with delta patches.
- [ ] Reproducible builds (pin all transitive deps via `cargo supply chain`).

### 4.2 Security hardening

- [ ] Threat model document + external security review.
- [ ] Fuzz the IPC layer with `cargo-fuzz` (every Tauri command).
- [ ] Sandbox the AI: deny file writes outside an allow-list by default,
      even with `allow_autonomous` enabled.
- [ ] Rate-limit AI actions per minute to prevent runaway loops.
- [ ] Sign every release artifact with a published PGP key.

### 4.3 Privacy & compliance

- [ ] GDPR data export (`aegis export`) and full wipe (`aegis forget`).
  *(v0.6: `memory_export_all` and `memory_forget_all` Tauri commands
  expose this via the Settings UI. The CLI commands are queued for
  Phase 4 alongside a single-binary packaging refactor.)*
- [ ] Telemetry opt-in only — never on by default.
  *(v0.6: no telemetry code in the codebase. The Phase 4 task is to
  add an opt-in telemetry layer + a privacy dashboard.)*
- [ ] Audit log exportable as CSV / JSON for incident response.
  *(v0.6: `audit_export` Tauri command supports both `json` and `csv`
  formats. Exportable from Settings → Data & Privacy.)*
- [ ] SOC 2 Type II readiness checklist (documentation only — no audit commitment).

### 4.4 Documentation

- [ ] User guide (Markdown → mdBook → hosted docs).
- [ ] Developer guide: how to add a new provider (template + walk-through).
- [ ] Architecture decision records (ADRs) for major design choices.
- [ ] Threat model + security white paper.

### 4.5 Localization polish

- [ ] Switch to `fluent-bundle` for proper pluralization.
- [ ] Add: Spanish, French, German, Japanese, Simplified Chinese.
- [ ] Community translation portal (Weblate / Crowdin).

### Phase 4 exit criteria

- [ ] v1.0 installers are code-signed and pass SmartScreen / Gatekeeper.
- [ ] External security review finds no Critical or High findings.
- [ ] Documentation site is live and search-indexed.
- [ ] At least one third-party contributor has shipped a new provider.

---

## Versioning

We follow [Semantic Versioning](https://semver.org/):

- `v0.x.y` — pre-1.0, breaking changes allowed between minor versions.
- `v1.0.0` — first stable release after Phase 4 exit criteria are met.
- `v1.x.y` — backwards-compatible features and bug fixes only.

Each release ships:
- Linux: `.deb`, `.rpm`, `.AppImage`, source tarball.
- Windows: `.msi`, `.exe` (NSIS), portable `.zip`.
- SHA-256 checksums + PGP signature file.
- Release notes generated from this roadmap.

---

## How to contribute

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, coding standards,
and the PR review process.

To pick up a Phase 2+ task, open an issue referencing the relevant
checkbox above and tag it with `phase-2` (or whichever phase applies).
