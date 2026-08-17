# Aegis AI — Development Roadmap

This file tracks the four-stage development plan for Aegis AI. Each phase has
a clear goal, scope, deliverables, and exit criteria. The current release is
**v0.1 (Phase 1 — Foundation Skeleton)**.

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

### 2.1 Streaming chat over Tauri events

- [ ] Expose `ai_chat_stream` as a true streaming command that emits
      `chat://chunk` events to the frontend.
- [ ] Frontend Chat component renders incremental deltas.
- [ ] Add a "Stop generation" button that cancels the underlying `reqwest`
      future via a `CancellationToken`.

### 2.2 Full computer-use agent

- [ ] Replace `computer/automation.rs` stubs with `enigo` for cross-platform
      mouse/keyboard.
- [ ] Implement `computer/screen.rs` with the `screenshots` crate for capture
      and `tesseract-rs` for OCR.
- [ ] Wire a "tool-use" loop: AI proposes an action → safety policy evaluates
      → user confirms → action runs → result fed back to AI.
- [ ] Add a `confirm_action` Tauri command backed by a token table
      (`HashMap<String, PendingAction>`) with 60-second expiry.

### 2.3 Real antivirus integration

- [ ] Linux: detect `clamdscan` on PATH; if present, delegate file scans
      to ClamAV (huge signature DB).
- [ ] Windows: invoke `MpManagerStartScan` via the Defender API
      (requires `MpClient.dll` FFI).
- [ ] Load ClamAV-style daily.cvd / main.cvd signature files into the local
      hash store when ClamAV is absent.
- [ ] Implement YARA rule loading (Phase 2.5 if time permits).

### 2.4 Cloud provider finishing touches

- [ ] Azure OpenAI: AAD token + `api-version` query param + deployment-id model.
- [ ] AWS Bedrock: SigV4 request signing with `aws-sigv4` crate.
- [ ] Replicate: implement the async Prediction API (POST /v1/predictions
      → poll until `succeeded` / `failed` → return output).

### 2.5 Persistence upgrade

- [ ] Move credentials out of `config.toml` into the OS keychain
      (`keyring` crate on Linux / Windows Credential Manager).
- [ ] Encrypt the SQLite database at rest with SQLCipher (opt-in).
- [ ] Implement conversation summarization (using the configured AI) to
      compress long histories into a compact context.

### 2.6 Better security signals

- [ ] Network anomaly detector: enumerate listening sockets via
      `procfs` (Linux) / `GetExtendedTcpTable` (Windows).
- [ ] File integrity monitor: SHA-256 hash of `~/.bashrc`, `~/.ssh/authorized_keys`,
      `~/.config/autostart/` and Windows Run keys.
- [ ] Email/webhook alerts for critical threats.

### Phase 2 exit criteria

- [ ] Real mouse-click through the AI (a test that types "open notepad"
      results in Notepad launching on Windows).
- [ ] ClamAV scan of the EICAR test file flags it as infected.
- [ ] Azure OpenAI and Bedrock return real chat completions.
- [ ] Credentials never appear in plaintext on disk.

---

## Phase 3 — Advanced Capabilities (v0.4 – v0.6)

**Goal:** Differentiate from generic AI chat apps with proactive agent
behavior, voice, and deep OS integration.

### 3.1 Event-driven continuous mode

- [ ] File-system watcher (`notify` crate) — AI reacts to new files in
      watched directories.
- [ ] Calendar integration (CalDAV) — proactive daily summaries.
- [ ] Calendar-intent dispatch ("schedule a meeting with…") via the AI.
- [ ] Wake-on-event bus: security escalations, calendar ticks, file events,
      hotkey presses.

### 3.2 Voice I/O

- [ ] Whisper-based local STT for "Hey Aegis" wake word + voice input.
- [ ] TTS playback via Piper (local) or ElevenLabs (cloud, opt-in).
- [ ] Push-to-talk hotkey registered system-wide.

### 3.3 Knowledge graph

- [ ] Replace the simple `key → value` knowledge table with a vector
      embedding store (`qdrant` or `lancedb`).
- [ ] Auto-extract entities from chat history (regex + LLM).
- [ ] Retrieval-augmented generation (RAG): inject relevant facts into the
      next chat's system prompt.

### 3.4 Tool/function calling

- [ ] OpenAI function-calling schema for structured tool invocation.
- [ ] Anthropic tool-use API.
- [ ] Local tool registry: file ops, web search, shell exec, screenshot.
- [ ] Tool-result truncation policy to stay under context window.

### 3.5 Mobile companion (stretch)

- [ ] Tauri Mobile (iOS + Android) build for read-only dashboard.
- [ ] End-to-end-encrypted sync of conversation history via a relay.

### Phase 3 exit criteria

- [ ] User says "Hey Aegis, what did I work on today?" — AI summarizes the
      day's activity using both chat history and OS events.
- [ ] RAG injects at least one remembered fact into a relevant conversation.
- [ ] Voice input round-trip works in <2s on commodity hardware.

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
- [ ] Telemetry opt-in only — never on by default.
- [ ] Audit log exportable as CSV / JSON for incident response.
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
