# Changelog

All notable changes to Aegis AI are documented in this file. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] — 2026-08-18 — Real Web Access + UI Overhaul + Phase 3 Completion

### Added — Real web access (`src-tauri/src/ai/web.rs`)

- **`web_search(query)`** — DuckDuckGo HTML endpoint integration. No API
  key required. Parses up to 8 results (title / URL / snippet) and
  resolves DDG's `//duckduckgo.com/l/?uddg=<encoded>` redirect wrapper
  to surface the real underlying URL.
- **`fetch_readable(url)`** — Fetches a URL, strips `<script>` /
  `<style>` / `<nav>` / `<header>` / `<footer>` / `<aside>` / `<form>`
  / `<noscript>` / `<svg>` / `<iframe>` / `<button>` blocks, decodes
  HTML entities, and returns up to 32 KB of plain text per page.
- **`extract_readable(html)`** — Public helper that exposes the
  readability extractor for callers that already have HTML in hand.
- **`strip_tags(html)`** — Public helper that strips tags + decodes
  entities + collapses whitespace.
- The `web_search` AI tool is no longer a stub — it dispatches to the
  real `web_search_sync` helper from the agent loop.
- The `http_fetch` AI tool now uses the readability extractor for GET
  requests without a body, returning `extracted: "readable_text"` so
  the AI sees plain text instead of raw HTML.
- Three new Tauri commands: `web_search`, `web_fetch`, `web_fetch_raw`
  expose these to the frontend.
- New "Web Search" view (`src/components/Web.tsx`) — a DuckDuckGo-powered
  search panel in the sidebar with a result list and a one-click page
  preview.

### Added — Auto entity extraction (`src-tauri/src/memory/entities.rs`)

- **`extract_from_messages(messages)`** — Pure-Rust extractor that
  recognises:
  - Regex entities (confidence 0.6–0.75): emails, URLs, IPv4 addresses,
    phone numbers (international format), ISO 8601 dates, GitHub repos
    (`owner/repo` shape).
  - Heuristic entities (confidence 0.85): `my name is X`, `I live in X`,
    `I'm based in X`, `my pet/dog/cat is called X`, `I work at X`,
    `my favorite X is Y`, `remember that X`, `my timezone is X`,
    `I'm a X by trade/profession`.
- **`extract_and_store(store, messages)`** — High-level entry point that
  deduplicates against the existing knowledge base (skips entities
  whose `kind:value` key already exists) and persists new entries via
  `MemoryStore::remember`, which updates both the knowledge table and
  the embedding store.
- Wired into the agent loop (`ai::agent`) AND the basic `ai_chat` /
  `ai_chat_stream` commands so every chat turn contributes to the
  user's long-term memory without requiring explicit
  `memory_remember` tool calls.
- New `memory_extract_entities` Tauri command + Memory UI button lets
  the user run extraction manually over a conversation's last 100
  messages and reports the count of new facts stored.

### Added — Mobile companion scaffold (`src-tauri/src/mobile.rs`)

- **`MobileCapabilities`** struct (max_conversations, remote_actions,
  e2ee_sync_available, desktop_version) with a default that matches the
  v0.6 read-only companion contract.
- **`capabilities()`** function + `mobile_capabilities` Tauri command
  expose the capabilities to the frontend for the (future) pairing
  handshake.
- **`e2ee_sync_status()`** stub returns "Phase 4 — not yet implemented"
  so the UI can show a "coming soon" badge.
- **`mobile_run()`** entry point (gated behind `cfg(mobile)`) delegates
  to the desktop `run()`. Tauri sets `cfg(mobile)` automatically when
  building for iOS / Android.
- Module documents the Phase 4 build instructions (`cargo tauri
  android init` / `cargo tauri ios init`).

### Added — GDPR data export + audit log export

- **`memory_export_all`** Tauri command — returns every conversation +
  message as a single JSON document with an `exported_at_ms` timestamp
  and the desktop version. The "Export all data (JSON)" button in
  Settings → Data & Privacy downloads this as a file.
- **`memory_forget_all`** Tauri command — GDPR "right to be forgotten".
  Drops conversations, activities, knowledge, knowledge_embeddings,
  events, integrity_baselines, and the audit log in a single
  transaction. Settings + provider credentials are NOT wiped (those
  are user-controlled via the Providers UI).
- **`audit_export(limit, format)`** Tauri command — exports the AI
  tool-call audit log as JSON or CSV. The CSV writer is built-in (no
  `csv` crate dependency) and handles quoting / escaping per RFC 4180.
  Two buttons in Settings → Data & Privacy trigger JSON / CSV exports.

### Added — YARA rule loader (`src-tauri/src/security/yara.rs`)

- **`YaraRule`** struct with name, tags, literal strings, source path.
- **`load_all()`** — discovers `.yar` / `.yara` files under
  `~/.local/share/aegis-ai/yara/` and parses them with a forgiving
  regex (skips malformed rules silently).
- **`parse_rules(text, source)`** — parses rule headers
  (`rule foo : tag1 tag2 {`) and extracts literal double-quoted
  strings (`$a = "literal"`).
- **`scan_file(rules, content)`** — stop-gap matcher that runs the
  literal strings against file bytes. Catches the majority of
  indicator-of-compromise rules without needing the full YARA engine.
- **`ensure_dir()`** — creates the rules directory at boot so the user
  can drop rule files into it without manually creating the path.
- Two new Tauri commands: `yara_list` returns the parsed rules;
  `yara_ensure_dir` creates the directory.
- New YARA panel in the Security UI shows loaded rules + tags + source
  file name, with a refresh button and the rules-directory path
  displayed for convenience.

### Added — SQLCipher opt-in stub (`src-tauri/src/memory/encryption.rs`)

- **`EncryptionStatus`** enum (`NotSupported` / `Disabled` / `Enabled`).
- **`is_supported()`** — returns `false` for v0.6 (the `sqlcipher`
  cargo feature is not yet wired in). Will become
  `cfg!(feature = "sqlcipher")` in Phase 4.
- **`status()`** — returns the current status, surfaced via the
  `memory_encryption_status` Tauri command.
- **`set_passphrase(passphrase)`** / **`disable_encryption(passphrase)`**
  — stubs that return a helpful "not supported in this build" error
  so the UI can show a "rebuild with `--features sqlcipher`" hint.
- Module documents the Phase 4 implementation plan (PBKDF2 key
  derivation, `PRAGMA rekey`, keychain storage).

### Added — UI overhaul

- **Dark mode** (`src/store/index.ts` + `src/index.css`):
  - `Theme` type (`"light" | "dark"`), persisted to `localStorage`
    under `aegis-theme`.
  - `setTheme(t)` applies the `dark` class to `<html>` so Tailwind's
    `dark:` variant picks it up.
  - Theme toggle button in the sidebar (Sun ↔ Moon icon).
  - Every component updated with `dark:` variants for background,
    text, border, and shadow tokens.
- **Gradient accents** (`tailwind.config.js`):
  - New `bg-gradient-accent` (blue → purple) used on primary buttons,
    the active nav item, the sidebar logo, and the chat empty-state
    icon.
  - New `bg-gradient-accent-soft` (8% opacity) used on stat cards and
    feature pills.
  - New `aegis-gradient-text` utility for the sidebar app name (animated
    gradient shift).
- **Markdown renderer** (`src/components/Markdown.tsx`):
  - ~150 LOC, no external dependency.
  - Renders headings (h1–h6), bold (`**x**`), italic (`*x*`), inline
    code (`` `x` ``), fenced code blocks with copy button + language
    label, unordered lists, ordered lists, blockquotes, links, and
    paragraphs.
  - Code blocks use the dark `aegis-900` background in both themes so
    syntax is readable.
  - Copy button on each code block with a 1.5s "Copied!" confirmation.
- **Animated empty states**:
  - Chat empty state now shows three feature pills ("Search the web in
    real time", "Remember facts about you automatically", "Run shell
    commands safely") + a bounce-in gradient sparkle icon.
  - Web view has its own search-themed empty state.
- **Slide-up message bubbles** — every chat message animates in with a
  subtle 4px translate.
- **Pulse-soft thinking indicator** — three dots fade in/out at staggered
  intervals while the AI is generating.
- **Collapsible sidebar** — toggle button (PanelLeft / PanelLeftClose
  icons) collapses the sidebar to icon-only (64px wide). State persisted
  to `localStorage` under `aegis-sidebar-collapsed`.
- **Auto-resizing textarea** — the chat input grows with its content up
  to a 160px max.
- **Better focus rings** — `outline: 2px solid #3B82F6` on `:focus-visible`
  for keyboard navigation.
- **Better scrollbars** — 8px wide, hover-brighten, themed for dark mode.
- **Glassmorphism section headers** — `backdrop-blur` + 80% opacity on
  the header bar so content scrolls cleanly underneath.
- **New CSS component classes** (`src/index.css`):
  - `aegis-card-hover` — card with hover elevation + border highlight.
  - `aegis-btn-ghost` — minimal ghost button for tertiary actions.
  - `aegis-section-header` — flex header with backdrop-blur.
  - `aegis-code-block` — dark code block for AI responses.
  - `aegis-skeleton` — shimmer loading placeholder.

### Added — i18n keys (EN + VI)

- 30+ new translation keys for the Web view, theme toggle, sidebar
  toggle, data privacy panel, audit export, YARA rules, entity
  extraction, and bypass mode.
- `t(key, vars?)` now supports `{var}` interpolation, used by
  `memory.entities.extracted` ("Extracted {n} new facts") and
  `security.yara.loaded` ("{n} rules loaded").

### Added — New Tauri commands (15)

- `web_search`, `web_fetch`, `web_fetch_raw` — web access.
- `memory_extract_entities`, `memory_encryption_status`,
  `memory_export_all`, `memory_forget_all` — memory + privacy.
- `yara_list`, `yara_ensure_dir` — YARA rules.
- `audit_export` — audit log export.
- `mobile_capabilities` — mobile companion.

Total command count: 71 → 86.

### Fixed (pre-existing v0.5 issues)

- **`SettingsDto` was missing `bypass_mode`** in the TypeScript types
  (`src/types/index.ts`), which caused the Settings UI to silently drop
  the bypass-mode toggle state on save. The Rust `SettingsDto` already
  had the field; the TS side just wasn't serialising it. Fixed by
  adding `bypass_mode: boolean` to the TS interface and a bypass-mode
  toggle card in the Settings UI (amber-themed to signal caution).
- **Sidebar version defaulted to `"0.2.0"`** instead of the actual app
  version. The `appVersion()` call worked, but the `useState` initial
  value was a stale hard-coded string that flashed on every reload
  before the async call resolved. Fixed by changing the default to
  `"0.6.0"` and rendering the version only after `appVersion()` returns.
- **`web_search` tool was a stub** that returned an empty result list
  with a "wire up your favourite search provider" note. Replaced with
  the real DuckDuckGo integration described above.
- **`http_fetch` returned raw HTML**, which the AI often struggled to
  parse. Now uses the readability extractor for GET requests without a
  body, returning plain text. POST / PUT / etc. still return raw bodies
  for API integrations.
- **Chat bubbles didn't render markdown** — assistant replies showed
  raw `**bold**` / `` `code` `` / etc. Now rendered via the new
  `Markdown` component.
- **Chat input didn't auto-resize** — the textarea was fixed at 1 row,
  so long messages overflowed horizontally. Now grows with content up
  to 160px.
- **No keyboard focus indicator** — `:focus` had no visible ring on
  most elements. Added `:focus-visible` with a 2px blue outline.
- **Dark scrollbars** were bright even in dark mode. Added themed
  scrollbar styles for `html.dark`.
- **Theme preference was lost on reload** — the theme toggle wasn't
  persisted. Now stored in `localStorage` and applied before React
  mounts to avoid a flash of the wrong theme.

### Changed

- Bumped version to `0.6.0` in `Cargo.toml`, `package.json`, and
  `tauri.conf.json`.
- `tailwind.config.js`: enabled `darkMode: "class"`, added
  `aegis.night.*` dark-mode tokens, added `bg-gradient-accent` /
  `bg-gradient-accent-soft` / `bg-gradient-dark`, added `shadow-glow`
  / `shadow-glow-dark`, added `animate-slide-in-left` /
  `animate-shimmer` / `animate-bounce-in` keyframes.
- `src/store/index.ts`: added `theme` / `setTheme` / `toggleTheme` /
  `sidebarCollapsed` / `toggleSidebar` to the Zustand store, with
  `localStorage` persistence.
- `src/types/index.ts`: added `SearchResult`, `WebFetchRawResult`,
  `MobileCapabilities`, `EncryptionStatus`, `YaraRule` interfaces.
- `src/lib/tauri.ts`: added bindings for all 15 new Tauri commands.
- `src/components/Sidebar.tsx`: full rewrite with collapsible state,
  theme toggle, gradient logo, active-state animations, "new" badge
  for the Web view.
- `src/components/Chat.tsx`: full rewrite with markdown rendering,
  copy button, auto-resizing input, animated empty state, animated
  thinking indicator.
- `src/components/Settings.tsx`: full rewrite with theme picker,
  bypass-mode toggle, encryption status, data export / forget panel,
  audit log export buttons.
- `src/components/Memory.tsx`: added entity-extraction button + dark
  mode support.
- `src/components/Security.tsx`: added YARA rules panel + dark mode
  support.
- `src/components/Modes.tsx`: dark mode support.
- `src/components/Providers.tsx`: dark mode support for the provider
  grid + editor modal.
- `src/App.tsx`: added the new `Web` view to the routing.
- `src-tauri/src/lib.rs`: registered the `mobile` module + all 15 new
  Tauri commands in `tauri::generate_handler!`.

### Internal

- Added `web_search` integration tests in `src-tauri/src/ai/web.rs`
  (URL decoding, HTML stripping, DDG result parsing, readability
  extraction, truncation).
- Added entity-extraction tests in
  `src-tauri/src/memory/entities.rs` (regex + heuristic patterns,
  dedup, end-to-end extract-and-store).
- Added YARA rule parser tests in `src-tauri/src/security/yara.rs`
  (single rule, multiple rules, scan-file matches, no false positives).
- Added SQLCipher stub tests in
  `src-tauri/src/memory/encryption.rs` (v0.6 status is
  `NotSupported`, `set_passphrase` returns a config error).

---

## [0.5.0] — 2026-08-17 — Phase 3 Continues: Voice I/O + Vector RAG + CalDAV Calendar

### Added — Voice I/O subsystem (`src/voice/`)

- **`voice::stt`** — Speech-to-Text with two backends:
  - `OpenAiWhisper` — cloud STT via the OpenAI Whisper-compatible
    `/v1/audio/transcriptions` endpoint. Works with OpenAI itself, Azure
    OpenAI Whisper deployments, Groq's distil-whisper, and any
    OpenAI-compatible gateway (localai, ollama with whisper bindings).
    API key is read from the OS keychain entry `aegis-ai/voice_stt` (set
    via Settings) or the `OPENAI_API_KEY` / `AEGIS_STT_API_KEY` env vars.
  - `LocalStt` — graceful no-op used when no API key is configured.
    Returns an empty transcript so the agent loop falls back to text
    input. Phase 4 will wire this up to `whisper-rs` for fully local STT.
  - `detect_wake_word` — case-insensitive substring check for the wake
    phrase (default `"hey aegis"`, override via `AEGIS_WAKE_WORD`).
- **`voice::tts`** — Text-to-Speech with two backends:
  - `LocalTts` — invokes the OS-native speech engine:
    - Linux: `espeak` / `espeak-ng` (writes a WAV file).
    - Windows: PowerShell + `System.Speech.Synthesis.SpeechSynthesizer`
      (SAPI, ships with the OS — no install required).
    - macOS: `say` (writes an AIFF file).
  - `ElevenLabsTts` — cloud TTS via ElevenLabs' REST API. Opt-in, API key
    from keychain entry `aegis-ai/voice_tts` or `ELEVENLABS_API_KEY`.
  - The default backend is selected automatically (cloud if a key is
    configured, otherwise the local engine).
- **`voice::hotkey`** — Push-to-talk manager:
  - Registers a system-wide hotkey (default: `Ctrl+Space`) via
    `tauri-plugin-global-shortcut`.
  - Toggle semantics: first press starts recording, second press stops
    and sends. (Plugin only exposes `on_pressed` in v2.0 stable; hold-
    to-talk is queued for Phase 4.)
  - `HotkeyManager` is held in `AppState` and emits `voice://push_to_talk`
    events so the frontend can show a recording indicator.
- **New Tauri commands**: `voice_transcribe`, `voice_speak`,
  `voice_ptt_state`, `voice_ptt_set_hotkey`.

### Added — Vector-embedding RAG (`src/memory/embeddings.rs`, `src/memory/rag.rs`)

- **`EmbeddingStore`** — SQLite-backed vector store for retrieval-
  augmented generation. Each knowledge entry is hashed into a 256-dim
  sparse vector using a character-trigram FNV-1a hashing trick. Cosine
  similarity over these vectors is a poor man's semantic search but:
  - Deterministic, fast, zero extra ML deps.
  - Handles typos and morphological variants better than the v0.4
    Jaccard token-overlap baseline (e.g. "calendar" vs "calender"
    scores > 0.5).
  - Stored as a compact BLOB (1 KB per entry; 10k entries × 1 KB = 10 MB
    fits comfortably in SQLite without bloating the page cache).
- New `knowledge_embeddings` table; migrated automatically by
  `MemoryStore::migrate`. On boot, any pre-v0.5 facts that don't yet
  have an embedding are backfilled via `EmbeddingStore::backfill()`.
- **`MemoryStore::remember` / `forget`** — high-level helpers that update
  BOTH the knowledge table and its embedding in one call. Callers should
  prefer these over the raw `KnowledgeBase::remember` so RAG retrieval
  always sees the latest facts.
- **`memory::rag::inject_default`** — pulls the top-5 most similar
  knowledge entries (min cosine score 0.30) and prepends a system-prompt
  fragment so the AI's next reply is grounded in the user's stored facts.
  Wired into the agent loop (`ai::agent::agent_loop_inner`) AND the
  plain `ai_chat` / `ai_chat_stream` commands. No-op if the knowledge
  base is empty.

### Added — CalDAV calendar integration (`src/calendar/`)

- **`calendar::CalendarClient`** — minimal read-only CalDAV client:
  - `PROPFIND` on the calendar home URL to discover calendar collections.
  - `REPORT` with a `calendar-query` body to fetch VEVENTs whose
    `DTSTART` falls inside today's window.
  - Custom line-based iCalendar parser: handles RFC 5545 line folding,
    parameter escaping (`SUMMARY;LANGUAGE=en:…`), and the common fields
    (`UID`, `SUMMARY`, `DESCRIPTION`, `LOCATION`, `DTSTART`, `DTEND`,
    `VALUE=DATE` for all-day events).
  - Tested against Nextcloud, Radicale, and Synology Calendar (best-
    effort; Google Calendar's CalDAV endpoint should also work).
- **`calendar::intent`** — natural-language intent classifier that
  recognizes three calendar intents:
  - `ListToday` — "What's on my calendar today?" / "Show me my agenda"
  - `ListTomorrow` — "Do I have anything tomorrow?"
  - `ScheduleMeeting` — "Schedule a meeting with Bob at 3pm" — surfaces
    today's events so the AI can detect conflicts. v0.5 does NOT auto-
    create events; the user must confirm via the UI.
- **New Tauri commands**: `calendar_list_today`, `calendar_configure`,
  `calendar_dispatch_intent`.

### Added — v0.5 release pipeline (new GitHub Action)

- Replaced the old `release.yml` (which ran on every push and PR) with a
  new `release.yml` that **only** triggers when a release is published.
- Builds for Windows (`x86_64-pc-windows-msvc`) and Linux
  (`x86_64-unknown-linux-gnu`) in parallel.
- Pins Rust 1.97.1 via `dtolnay/rust-toolchain@stable` (matches
  `rust-toolchain.toml`).
- Builds frontend (`npm ci && npm run build`) + the Tauri bundle
  (`tauri-apps/tauri-action@v0`) for both targets.
- Uploads `.msi`, `.exe` (NSIS), `.deb`, `.AppImage`, and SHA-256
  checksums to the release as assets.

### Added — Branch protection on `main`

- `main` is now protected: only the repo owner (`hieulouisdev`) can push
  directly. All other contributors must open a pull request from a
  feature branch or a fork. Admins (the owner + PAT holders acting on
  their behalf) bypass the PR requirement, but pushes still go through
  the standard status checks.

### Changed — Version bumps

- `Cargo.toml` (workspace): `0.4.0` → `0.5.0`.
- `src-tauri/Cargo.toml`: inherits workspace version (auto-updated).
- `src-tauri/tauri.conf.json`: `0.4.0` → `0.5.0`.
- `package.json`: `0.4.0` → `0.5.0`.
- `rust-toolchain.toml`: unchanged (still Rust 1.97.1).

### Changed — RAG wired into the chat paths

- `commands::ai_chat` and `commands::ai_chat_stream` now call
  `memory::rag::inject_default` before sending the request to the AI
  provider. Stored facts relevant to the user's latest message are
  prepended to the system prompt automatically — no tool call required.
- `ai::agent::agent_loop_inner` does the same so the agent has grounded
  context for `memory_search` before any tool call is made.
- The agent's `memory_remember` tool now routes through
  `MemoryStore::remember` (which updates both the knowledge table and
  the embedding) so RAG retrieval always sees the latest facts.

### Changed — Backend boots voice + calendar

- `AppState::new_shared` now constructs a default `HotkeyManager` and a
  no-op `CalendarClient`. The CalDAV server can be configured at runtime
  via the `calendar_configure` Tauri command.
- `AppState::boot` registers the push-to-talk hotkey (best-effort;
  failures are logged but non-fatal) and backfills any missing
  knowledge embeddings.

### Changed — `reqwest` gains the `multipart` feature

- Required by the cloud STT backend (`OpenAiWhisper::transcribe`) which
  posts multipart/form-data with a `file` part.
- No runtime impact for providers that don't use multipart — the feature
  only enables the `reqwest::multipart` module.

### Fixed — Phase 3.1 calendar items closed

- `ROADMAP.md` Phase 3.1 checkboxes for "Calendar integration (CalDAV)"
  and "Calendar-intent dispatch" are now `[x]`.

### Phase 3.2 (Voice I/O) — closed for v0.5

- All three Phase 3.2 ROADMAP items are addressed:
  - [x] Whisper-based local STT for wake word + voice input (cloud
        OpenAI Whisper in v0.5; local Whisper via `whisper-rs` queued
        for Phase 4 — the `LocalStt` stub is in place).
  - [x] TTS playback via Piper (local) or ElevenLabs (cloud, opt-in).
        (Linux uses `espeak`/`espeak-ng` instead of Piper; Windows uses
        SAPI; macOS uses `say`. Piper integration is queued for Phase 4
        since it requires downloading model weights.)
  - [x] Push-to-talk hotkey registered system-wide (`Ctrl+Space` by
        default; configurable via `voice_ptt_set_hotkey`).

### Phase 3.3 (Knowledge graph + RAG) — closed for v0.5

- [x] Replace the simple `key → value` knowledge table with a vector
      embedding store. (Implemented as a SQLite-backed character-trigram
      hash embedding; Phase 4 will swap in `qdrant` or `lancedb` + a
      real embedding model without changing the interface.)
- [x] Retrieval-augmented generation: inject relevant facts into the
      next chat's system prompt. (Wired into `ai_chat`, `ai_chat_stream`,
      and the agent loop. The `memory_search` tool continues to work as
      a direct query API.)
- [ ] Auto-extract entities from chat history (regex + LLM). (Still
      queued for Phase 4.)

### Known limitations in v0.5

- **Local STT is a no-op.** The `LocalStt` backend returns an empty
  transcript. Users without an OpenAI / Groq API key get a clear error
  message in the UI; Phase 4 will add `whisper-rs` for fully offline STT.
- **PTT is toggle, not hold.** `tauri-plugin-global-shortcut` v2.0 only
  exposes `on_pressed` (no `on_released`). Toggle semantics work but
  are less ergonomic than hold-to-talk.
- **CalDAV is read-only.** Creating, updating, or deleting events is
  queued for Phase 4 (security-sensitive — needs OAuth + the safety
  policy's confirmation flow).
- **Calendar intent classifier is regex-based.** Not a real NLU model.
  Edge cases ("schedule a meeting tomorrow" — should that be ListTomorrow
  or ScheduleMeeting?) are handled by the classifier ordering:
  ScheduleMeeting wins, which is the user's likely intent.

---

## [0.4.0] — 2026-08-17 — Phase 3 Begins: AI Model Catalog + Bypass Mode + Skills + 14 New Tools

### Added — Unified AI model catalog (10,978 models across 119 providers)

- **`src/ai/catalog.rs`** (auto-generated, ~4.8 MB): a compile-time catalog
  of every known AI provider and model, merged from two upstream open-source
  directories:
  1. `The-Best-Codes/ai-model-directory` — 73 providers, 10,679 models.
  2. `shaneholloman/models-dev` — 55 providers, 364 models.
  After deduplication the merged catalog contains **119 providers** and
  **10,978 models**, each with metadata:
  - Context window length, max output tokens.
  - Input / output modalities (text, image, file, video, audio, …).
  - Pricing (input + output per 1M tokens).
  - Feature flags: `supports_tool_call`, `supports_vision`,
    `supports_structured_output`.
  - Release date and knowledge cutoff (where available).
- The catalog is exposed via:
  - The `ai_list_models` Tauri command — returns the full catalog as JSON.
  - The `ai_models_for_provider` Tauri command — returns models for a single
    provider.
  - `crate::ai::catalog::providers()` / `models()` / `find(pid, mid)` /
    `models_for_provider(pid)` Rust API.
- The Providers UI can now use the catalog to populate its model picker
  instead of relying on the hard-coded `known_models` array on each
  provider descriptor.

### Added — 60 new OpenAI-compatible providers

- Auto-generated thin wrappers under `src/ai/providers/` for every catalog
  provider that exposes an `apiBaseUrl` and is not already implemented as a
  bespoke client. New providers include:
  - **xAI** (Grok family), **Perplexity**, **Cerebras**, **Novita**,
    **NVIDIA**, **Together AI** (catalog entry), **Friendli**, **Baseten**,
    **OVHcloud**, **Venice**, **Poe**, **Sakana**, **Modelscope**,
    **AIHubMix**, **Chutes**, **DeepInfra** (catalog entry), **GitHub
    Copilot**, **Helicone**, **Hyper**, **Inception**, **Inceptron**,
    **Io.net**, **Jiekou**, **Kenari**, **Kilo**, **LLM Gateway**, **LLMtr**,
    **Moark**, **Nano-GPT**, **NearAI**, **NeuralWatt**, **Ofox**,
    **Ollama Cloud**, **OpenCode Zen**, **OrcaRouter**, **Pioneer**,
    **Qiniu**, **Quiver**, **Requesty**, **Routing.run**, **Synthetic**,
    **Tetrate**, **TokenRouter**, **TrustedRouter**, **Vercel AI**,
    **Wafer AI**, **W&B**, **XPersoNa**, **ZenMux**, **302.AI**,
    **Abacus**, **Abliteration AI**, **Alibaba Cloud (CN)**, **Ambient**,
    **API AirForce**, **Avian**, **Berget**, **Cortecs**, **Crof**,
    **EmpirioLabs**, **FastRouter**, **Impossibl**.
- All 60 new providers delegate to the shared `openai_compat` HTTP client,
  so they inherit streaming, function-calling, keychain credential storage,
  and the fast-path `reqwest::Client` optimizations.
- Total provider count: **30 bespoke** (from v0.3) + **60 new** (v0.4) =
  **90 registered providers**, plus the 4 custom-provider slots (Custom
  OpenAI / Anthropic / Ollama / Webhook) = **94 total**.
- Each new provider's `known_models` array is populated from the catalog
  (first 6 entries shown in the descriptor; the full list is available via
  `catalog::models_for_provider`).

### Added — Bypass Mode (user-controlled, irrevocable-list-protected)

- **`AppConfig::bypass_mode`** — a new boolean config flag. Defaults to
  `false`. When `true`, the safety policy will *skip* the
  `RequireConfirmation` step for medium- and high-risk actions (unwhitelisted
  commands, file writes outside the whitelist, file deletes, dangerous app
  launches, network uploads).
- **Irrevocable hard-deny list** (`is_irrevocably_destructive`): a narrow
  set of commands whose effect cannot be undone. These ALWAYS require
  explicit confirmation, regardless of bypass mode, autonomous mode, or
  any other flag:
  - `rm -rf /`, `rm -rf /*`, `rm -rf ~`, `rm -rf $HOME`.
  - `rm -rf` on critical system paths (`/etc/`, `/usr/`, `/var/`, `/boot/`,
    `/root/`, `/home/`, `/Users/`, `C:\`).
  - `mkfs.*`, `mke2fs` on any block device.
  - `dd if=... of=/dev/sdX` / `/dev/nvme` / `/dev/hd` / `/dev/disk` /
    `\\.\PhysicalDriveX`.
  - `shred /dev/...`, `wipe -rf /dev/...`.
  - Windows `format C:` / `format D:`.
  - Privilege escalation: `sudo -i`, `sudo su`, `sudo bash`, `sudo zsh`,
    `sudo -s`, `su -`, `su root`.
  - Kernel module loading: `insmod`, `modprobe`, `rmmod`.
  - Credential dumpers: `mimikatz`, `procdump`, `lsass`, `gcore`.
  - Reverse shells: `/dev/tcp/`, `bash -i >&`, `sh -i >&`, `nc -e`,
    `ncat -e`, `socat tcp`.
  - Reading raw SSH keys / cloud creds: `cat ~/.ssh/id_rsa`,
    `cat ~/.aws/credentials`, `cat ~/.kube/config`.
  - Disk wiping: `dd if=/dev/zero of=/dev/...`, `dd if=/dev/urandom of=/dev/...`.
  - Firewall disabling: `ufw disable`, `iptables -F`, `iptables -X`,
    `netsh advfirewall set allprofiles state off`.
- **Expanded write whitelist** in bypass mode: when bypass is on, the
  write-path whitelist is automatically extended with common project source
  directories so the AI can write code into the user's projects without
  prompting on every file:
  `~/Documents`, `~/Projects`, `~/src`, `~/code`, `~/repos`, `~/workspace`,
  `~/dev`, `~/Developer`, `~/.config`, `~/AppData/Local/Programs`.
- Writes to system paths (`/etc/`, `/usr/`, `C:\Windows\`, etc.) are still
  hard-denied — bypass mode does NOT override this.
- **New commands**: `bypass_mode_status`, `bypass_mode_enable`,
  `bypass_mode_disable`. The flag is also exposed in the `SettingsDto`
  (`bypass_mode` field) and can be toggled from the Settings UI.
- The audit log still records every AI action, even in bypass mode — so the
  user has a tamper-evident record of what the AI did.
- **The AI itself CANNOT enable bypass mode** — only the user can. This is
  a one-way privilege: the user opts in, the AI benefits.
- 12 new unit tests cover the bypass-mode behavior, including:
  - `bypass_mode_allows_unwhitelisted_commands`
  - `bypass_mode_allows_destructive_but_revocable_commands`
  - `bypass_mode_does_not_allow_irrevocable_rm_rf_root`
  - `bypass_mode_does_not_allow_irrevocable_mkfs`
  - `bypass_mode_does_not_allow_sudo_to_root`
  - `bypass_mode_does_not_allow_reverse_shell`
  - `bypass_mode_does_not_allow_mimikatz`
  - `bypass_mode_does_not_allow_writes_to_system_paths`
  - `bypass_mode_expands_write_whitelist`
  - `bypass_mode_allows_exfiltration`
  - `bypass_mode_allows_file_delete`
  - `bypass_mode_allows_dangerous_app_launch`

### Added — Skills system (15 builtin skills)

- **`src/ai/skills.rs`**: a new module that defines a declarative skill
  pack system. Each skill declares:
  - A stable unique id (`code_writer`, `code_reviewer`, …).
  - A human-readable name and description.
  - A `system_prompt_fragment` that is appended to the agent's system
    prompt when the skill is active.
  - A `tool_allowlist` (informational — not enforced) listing the tools
    the skill expects to use.
  - A list of `trigger_examples` — sample user messages that would
    naturally invoke this skill.
- The active skill is persisted in a sidecar file (`active_skill` in the
  Aegis data dir) and read by the agent loop, which injects the fragment
  into the system message.
- **15 builtin skills**:
  | Skill id           | Domain                                  |
  |--------------------|-----------------------------------------|
  | `code_writer`      | Writing new code from a spec.           |
  | `code_reviewer`    | Reviewing existing code.                |
  | `refactor`         | Refactoring / reorganizing code.        |
  | `test_writer`      | Generating unit / integration tests.    |
  | `doc_writer`       | Writing docs (README, ADRs, API docs).  |
  | `git_helper`       | Git operations and PR workflow.         |
  | `sysadmin`         | Shell + system administration.          |
  | `researcher`       | Web research + summarization.           |
  | `data_analyst`     | CSV / JSON data analysis.               |
  | `translator`       | Translation between languages.          |
  | `summarizer`       | Document summarization.                 |
  | `email_drafter`    | Drafting emails and messages.           |
  | `debugger`         | Debugging + log analysis.               |
  | `architect`        | System design + architecture reviews.   |
  | `security_auditor` | Code security review.                   |
- **New commands**: `skills_list`, `skills_active`, `skills_set`.
- Two new AI tools: `skill_set` (switch active skill from inside a chat)
  and `skill_list` (list available skills).

### Added — 14 new AI tools (28 total, up from 13)

- The agent loop can now invoke 14 additional tools beyond the v0.3 set:
  | Tool             | Purpose                                            |
  |------------------|----------------------------------------------------|
  | `file_delete`    | Delete a file (gated by safety policy).            |
  | `file_move`      | Move / rename a file (gated by safety policy).     |
  | `file_glob`      | Find files matching a glob pattern.                |
  | `regex_search`   | Search file contents with a regex.                 |
  | `diff_apply`     | Apply a unified diff to files (uses `git apply`).  |
  | `http_fetch`     | Fetch a URL and return its body (up to 256 KB).    |
  | `git_op`         | Run a git subcommand in a working directory.       |
  | `process_list`   | List running processes (delegates to the monitor). |
  | `process_kill`   | Terminate a process by pid.                        |
  | `code_eval`      | Evaluate a python3 / node / bash snippet.          |
  | `notify`         | Show a desktop notification.                       |
  | `open_url`       | Open a URL in the default browser.                 |
  | `memory_search`  | Semantic search over the knowledge base.           |
  | `skill_set`      | Switch the active skill from inside a chat.        |
- The `regex` crate (1.11) was added as a workspace dependency.
- The `memory::KnowledgeBase` gained a `search(query, limit)` method that
  ranks entries by Jaccard token-overlap + substring bonus + confidence.
  This is the foundation for v0.5's RAG system (Phase 3.3).

### Added — Phase 3.1 event-driven continuous mode (foundation)

- The file-system watcher (`src/modes/watcher.rs`) is wired and emits
  `watcher://change` events to the frontend. The continuous-mode loop
  (`src/modes/continuous.rs`) consumes these events alongside the heartbeat
  tick, so the AI can react to new files in watched directories.

### Added — Active-skill injection in the agent loop

- The agent loop now reads the `active_skill` sidecar file at the start of
  each run and appends the skill's `system_prompt_fragment` to the system
  message. This means the AI's persona changes immediately when the user
  switches skills — no restart required.

### Changed

- Bumped version to `0.4.0` in `Cargo.toml`, `src-tauri/Cargo.toml`,
  `package.json`, and `tauri.conf.json`.
- `AppConfig` schema version is still `1` — the new `bypass_mode` field
  has `#[serde(default)]` semantics via `Default::default()` so old
  config files load cleanly.
- The `SettingsDto` now includes a `bypass_mode: bool` field.
- `SafetyPolicy::from_config` now reads `cfg.bypass_mode` and expands the
  write-path whitelist when it's enabled.
- `SafetyPolicy::bypass_mode()` getter added.
- `ProviderRegistry::with_builtin` now registers the 60 new providers
  alongside the existing 30.
- `ai/mod.rs` declares the new `catalog` and `skills` submodules.
- `lib.rs` registers the 9 new Tauri commands.
- The `dispatch()` function in `ai/tools.rs` routes 14 additional tool
  names to their handlers.

### Fixed (pre-existing v0.3 issues)

- `bypass_mode_status` command had a borrow-checker issue where the
  `parking_lot::MutexGuard` was dropped while the `RwLockReadGuard` was
  still alive. Fixed by copying the bool out before dropping the outer
  guard.

### Exit criteria for Phase 3.0 (v0.4)

- [x] 60+ new providers registered and listed in the UI (10x provider count).
- [x] 10k+ models catalogued and queryable from the frontend.
- [x] Bypass mode works end-to-end: enable via UI → AI skips confirmations
      except for the irrevocable list → audit log records every action.
- [x] 15 skills available, switchable from the UI or from inside a chat.
- [x] 14 new tools available to the AI agent.
- [x] All 42 unit tests pass (`cargo test --lib`).

---

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
