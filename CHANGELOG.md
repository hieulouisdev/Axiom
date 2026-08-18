# Changelog

All notable changes to Aegis AI. Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
