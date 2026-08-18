# Changelog

All notable changes to Aegis AI. Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [1.0.0] — 2026-08-19 — General-Availability Release

This is the first stable, production-tagged release of Aegis AI. Every
public surface (Rust backend, TypeScript frontend, GitHub Actions pipeline,
documentation) has been audited and fixed in a single comprehensive sweep.

### Fixed — Rust backend

- **`tauri.conf.json` schema** — the `bundle.plugins.updater` block was at
  the wrong nesting level (Tauri 2.6+ rejects unknown fields at the bundle
  root). Updater config moved to a top-level `plugins.updater` block so
  `tauri::generate_context!()` no longer panics at build time.
- **`commands.rs` `SharedState` import** — the v0.7 sandbox & telemetry
  command handlers referenced `SharedState` without importing it, causing
  7 hard compile errors. Added `use crate::SharedState;` to the import
  block.
- **`commands.rs` `Result<T, String>` conflict** — the workspace's
  `crate::error::Result<T>` type alias only takes one generic argument,
  so the sandbox & telemetry commands' `Result<X, String>` signatures
  failed to compile. Switched to `std::result::Result<X, String>` for
  those seven handlers.
- **`security/sandbox.rs` missing `dirs` crate** — three call-sites used
  `dirs::home_dir()` but the `dirs` crate was not declared in
  `Cargo.toml`. Added `dirs = "5.0"` to the workspace dependencies.
- **`openai_compat.rs` `maybe_api_key` returned the API key even for
  providers that don't require one** — the `if/else` branches were
  identical (`self.creds().api_key.clone()` in both arms). Now returns
  `None` when `requires_api_key` is false, so local providers (Ollama,
  LM Studio, …) no longer leak stored credentials to the bearer-auth
  header.
- **`voice/tts.rs` had three identical branches** for Linux, Windows,
  and "default" — all produced `.wav` files. macOS produces AIFF, the
  others produce WAV. Collapsed the redundant branches into a single
  `if cfg!(target_os = "macos") { "aiff" } else { "wav" }` lookup.
- **`apply_diff_minimal` was a stub** — the function counted hunks but
  never applied them, leaving the patched file unchanged. Replaced with
  a real unified-diff applier that handles `@@ -a,b +c,d @@` hunk
  headers, context lines, +/- adds and removes, and `\ No newline at
  end of file` markers.
- **`run_code_eval` ignored its `timeout_seconds` argument** — AI-
  generated code could hang the agent indefinitely. Added
  `run_with_timeout()` that spawns the interpreter and kills it after
  the deadline, returning `ErrorKind::TimedOut`.
- **`extract_heuristic` regex used `(?i)` over the whole pattern** —
  this made the `[A-Z]` anchor in the capture group match lowercase
  letters too, so "my name is Louis and I live in Hanoi" captured
  "Louis and" instead of "Louis". Switched to `(?i:prefix)` so only
  the keyword is case-insensitive; the capture stays case-sensitive.
- **`calendar/caldav.rs::unfold_lines` stripped the leading whitespace**
  from continuation lines, corrupting any iCal value that contained a
  space at the fold point (`SUMMARY:Hello\r\n World` became
  `SUMMARY:HelloWorld` instead of `SUMMARY:Hello World`). Now keeps a
  single separator space at the fold point, matching what producers
  actually wrote.
- **`memory/embeddings.rs` only used character trigrams** — RAG queries
  like "what is my dog's name?" failed to retrieve the stored fact
  "pet_name Rex dog" because the cosine similarity (0.23) was below
  the 0.30 threshold. Added word-unigram features (only for multi-word
  inputs, so single-word typo tolerance is preserved) so semantic
  matches cross the threshold.
- **`memory/entities.rs` compiled a regex in a tight loop** — the
  `favorite (\w+) is` pattern was being re-compiled on every outer
  capture. Lifted to a single compile outside the loop.
- **`ai/agent.rs` used `.min().max()` instead of `.clamp()`** —
  replaced with `clamp(1, ABSOLUTE_MAX_ITERATIONS)` for clarity.
- **`ai/providers/bedrock.rs` had an unused `client: Client` field**
  on `BedrockProvider` (the SigV4 signing path is stubbed out). Removed
  the field and its construction.
- **`security/network.rs::parse_addr` was flagged dead code** — the
  function is only called from `#[cfg(windows)]`. Added the same
  cfg-gate to the function definition.
- **`commands.rs::CsvWriter::to_string` shadowed the `ToString` trait
  method** — replaced with a `Display` impl so callers can use
  `format!("{wtr}")` or `wtr.to_string()` interchangeably.
- **`computer/safety.rs::expand_tilde` stripped a prefix manually** —
  replaced `p.starts_with("~/")` + `&p[2..]` with `strip_prefix("~/")`.
- **`calendar/caldav.rs::ymd_hms_to_unix` indexed `mdays` with a
  loop counter** — replaced `for m in 0..n { mdays[m] }` with
  `for (m, &dm) in mdays.iter().enumerate().take(n)`.
- **30+ unused imports / dead variables** — cleaned up across all
  modules so `cargo clippy --all-targets -- -D warnings` passes clean.

### Fixed — GitHub Actions

- **`build-release.yml` & `release.yml` referenced wrong target
  directory** — the workspace `Cargo.toml` is at the repo root, so
  build artifacts end up under `target/<target>/release/bundle/`, not
  `src-tauri/target/...`. The checksums step looked in the wrong path
  and silently produced zero checksum files. Both workflows now walk
  both paths defensively.
- **`continue-on-error: true` on `cargo fmt` and `cargo clippy` steps
  let lint regressions merge silently** — both are now hard gates that
  fail the build. Added a separate `lint-and-test` job that runs fmt,
  clippy, and `cargo test --lib` before any binary is built.
- **`release.yml` cache key included `src-tauri/target` and `target`
  but the cache restore-keys list was missing `cargo-release-${label}-`**
  — fixed so warm caches actually hit.
- **Build summary referenced `src-tauri/target/`** in its `find`
  invocation — updated to walk both `target/` and `src-tauri/target/`.

### Fixed — Frontend

- **`Sidebar.tsx` showed a stale version string ("0.8.0")** as fallback
  when the backend wasn't reachable — updated to `1.0.0` to match the
  `Cargo.toml` workspace version.
- **`Settings.tsx` had an unused `Loader` import shadowed by `Loader2`**
  — TypeScript still compiled because of `noUnusedLocals: false`, but
  the runtime was carrying dead code. Removed.
- **`tsconfig.json` `noUnusedLocals` / `noUnusedParameters` were both
  off** — left as-is for now (turning them on would be a breaking
  change for the inline-style `theme` variable in `Web.tsx`), but the
  unused imports that *were* there have been removed.

### Changed

- **Version bumped to 1.0.0** across `Cargo.toml`, `package.json`,
  `src-tauri/tauri.conf.json`, `Sidebar.tsx` fallback, and README badge.
- **Rust toolchain pinned to 1.97.1** — verified by `rustc --version`
  and the `rust-toolchain.toml` file.
- **`tauri.conf.json`** — updater `pubkey` left empty (signing keys are
  a v1.1 deliverable; the updater plugin degrades gracefully without
  one).

### Verified

- `cargo check --manifest-path src-tauri/Cargo.toml` — passes (0 errors).
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --
  -D warnings` — passes (0 warnings, 0 errors).
- `cargo test --manifest-path src-tauri/Cargo.toml --lib` — passes
  (104 tests, 0 failures).
- `cargo fmt --all -- --check` (in `src-tauri/`) — passes.
- `npm run build` (Vite frontend) — passes; produces `dist/` with
  index.html, CSS, and JS bundle.
- `npx tsc --noEmit` — passes (0 TypeScript errors).

## [0.9.0] — 2026-08-19 — Documentation & Polish

### Changed

- **All documentation rewritten** for clarity, conciseness, and professional presentation.
- README now includes screenshots and a feature table.
- Rust version pinned to 1.97.1 across all docs.

## [0.8.0] — 2026-08-18 — Bug-sweep & i18n Completion

### Fixed

- **GitHub Actions workflow** — malformed YAML (`branches: ain]`) silently disabled the pipeline; rewritten to `branches: [main]` with aligned per-OS job matrix.
- **i18n Locale expansion** — backend `Locale` enum expanded from 2 to 7 variants (`En`, `Vi`, `Es`, `Fr`, `De`, `Ja`, `ZhCn`). Full 7-locale pipeline works end-to-end.
- **Boot panic in `state.rs`** — locale fallback now returns `En` instead of panicking on unknown codes.
- **Quarantine panel** — restored in Security view after v0.7 regression.
- **Markdown link XSS** — sanitized `href` attributes to prevent `javascript:` URL injection.
- **`rust-toolchain.toml`** — pinned to Rust 1.97.1.

## [0.7.0] — 2026-08-10 — Skills, RAG, Voice, Bypass Mode

### Added

- **15 builtin Skills** — code writer, reviewer, refactorer, test writer, doc writer, git helper, sysadmin, researcher, data analyst, translator, summarizer, email drafter, debugger, architect, security auditor.
- **RAG (Retrieval-Augmented Generation)** — vector-embedding knowledge base with character-trigram hash embeddings, cosine similarity ≥ 0.30, top-5 retrieval prepended to system prompt.
- **Voice I/O** — push-to-talk (`Ctrl+Space`), cloud Whisper STT, OS-native + ElevenLabs TTS.
- **Bypass Mode** — user-controlled; skips confirmations for Medium/High except irrevocable hard-deny list (rm -rf /, mkfs, dd, sudo, reverse shells, kernel modules, credential dumpers).
- **+14 AI tools** (total 28) — file move/delete/glob/regex-search/diff-apply, git_op, process_list/kill, code_eval (python3/node/bash), notify, open_url, memory (remember/lookup/search), skill_set/list.
- **CalDAV calendar** — connect Nextcloud/Radicale/Synology; query "what's on my calendar today?".
- **5 new languages** — Español, Français, Deutsch, 日本語, 简体中文 (frontend only in v0.7; backend in v0.8).

### Changed

- Expanded write-path whitelist: `~/Projects`, `~/src`, `~/code`, `~/repos`, `~/Documents/AegisAI/`.
- Irrevocable hard-deny list now enforced regardless of bypass mode.

## [0.6.0] — 2026-07-25 — Provider Catalog & Fast-Path HTTP

### Added

- **Unified AI model catalog** — 10,978 models across 119 providers with context window, modalities, pricing, feature flags. Queryable from frontend via `ai_list_models`.
- **Fast-path HTTP** — tuned reqwest pool + LRU response cache + dedup layer; first-token latency reduced from ~1.5s to <400ms on warm calls.
- **+10 providers** — xAI (Grok), Perplexity, Cerebras, Novita, NVIDIA, Friendli, Baseten, OVHcloud, Venice, Poe, Sakana, Modelscope, AIHubMix, GitHub Copilot, Vercel AI.

### Fixed

- Provider streaming timeout on slow connections.
- Rate limiter race condition under concurrent requests.

## [0.5.0] — 2026-07-10 — Embeddings & Knowledge Base

### Added

- **Character-trigram hash embeddings** — 256-bucket sparse vectors, FNV-1a hash, L2-normalized, cosine similarity search.
- **Knowledge base** — key-value entries with source attribution, confidence scores, semantic search.
- **Entity extraction** — named entities from conversations (people, organizations, dates, locations).

## [0.4.0] — 2026-06-20 — Computer-Use Agent

### Added

- **Agent loop** — AI autonomously chains tools via OpenAI-style function-calling with iteration cap, kill switch, rate limiter.
- **14 AI tools** — exec_command, file_read/write/list, open_app, screenshot, clipboard_read/write, http_fetch, web_search, automate (GUI).

### Changed

- Safety policy upgraded to 5-level risk classifier with hard-deny list.

## [0.3.0] — 2026-06-01 — Security Hardening

### Added

- Auto-defense: passive process monitor → threat signatures → quarantine + kill.
- File scanner with SHA-256 hash checking.
- Audit log for all AI tool calls.

## [0.2.0] — 2026-05-15 — Multi-Provider & Modes

### Added

- 20+ AI providers (OpenAI, Anthropic, Gemini, Ollama, DeepSeek, Groq, Mistral, Cohere, Together, …).
- Continuous and On-demand operational modes.
- Bilingual UI (English + Vietnamese).

## [0.1.0] — 2026-05-01 — Initial Release

### Added

- Tauri 2.0 desktop shell with React frontend.
- Rust 1.97.1 backend with SQLite storage.
- Basic chat with streaming SSE.
- 5 core providers: OpenAI, Anthropic, Gemini, Ollama, DeepSeek.
- Safety policy: risk classifier + confirmation flow.
- Security monitor: process scanning + threat signatures.
