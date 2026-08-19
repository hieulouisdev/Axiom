# Aegis AI — Development Roadmap

Current release: **v1.1.0** (Toolchain & Dependency Modernization). The project follows a 4-phase plan.

---

## Phase 1 — Foundation (v0.1) ✅

Ship a compilable, runnable cross-platform desktop app with the full module layout.

**Status:** Released as `Aegis AI v0.1`.

| Subsystem | Status | Notes |
|---|---|---|
| Tauri 2.0 shell | ✅ | Native window, IPC, white-themed React UI |
| Rust 1.97.1 toolchain | ✅ | `rust-toolchain.toml` |
| AI provider trait + router | ✅ | `ChatRequest`, `ChatResponse`, `Provider`, `ProviderRegistry` |
| 5 core providers | ✅ | OpenAI, Anthropic, Gemini, Ollama, DeepSeek |
| 15+ OpenAI-compat providers | ✅ | Shared `openai_compat` HTTP code |
| Custom providers (4) | ✅ | OpenAI-compat, Anthropic-compat, Ollama-compat, webhook |
| Computer-use actions | ✅ | Shell, file I/O, app launch, screenshot |
| Safety policy | ✅ | 5-level risk classifier + confirmation flow |
| Security monitor | ✅ | Process scan every 15s, threat signatures, quarantine |
| SQLite memory store | ✅ | Conversations, activities, knowledge |
| i18n | ✅ | English + Vietnamese |
| CI/CD | ✅ | GitHub Actions: build + test on Linux + Windows |

---

## Phase 2 — Production Quality (v0.2–v0.3) ✅

Harden for real-world use: more providers, operational modes, auto-defense.

**Status:** Released as `Aegis AI v0.3`.

- [x] 20+ AI providers
- [x] Continuous and On-demand modes
- [x] Auto-defense (notify → quarantine → kill)
- [x] File scanner (SHA-256)
- [x] Audit log for all AI tool calls
- [ ] OS keychain for API keys (`keyring` crate) — *partially done, config.toml fallback remains*
- [ ] Signed confirmation tokens (HMAC, 60s expiry)
- [ ] File integrity monitor (SHA-256 baselines)

---

## Phase 3 — Intelligence (v0.4–v0.5) ✅

Computer-use agent, RAG, embeddings, knowledge base.

**Status:** Released as `Aegis AI v0.5`.

- [x] Agent loop (function-calling, iteration cap, kill switch)
- [x] 14 AI tools (shell, file I/O, apps, screenshot, clipboard, http, web)
- [x] Character-trigram hash embeddings (extended with word-unigram features in v1.0)
- [x] Knowledge base with semantic search
- [x] Entity extraction (case-sensitive capture fixed in v1.0)
- [ ] Neural embeddings (`all-MiniLM-L6-v2` via `ort`) — *planned v1.1*
- [ ] YARA rule scanning — *partially implemented*

---

## Phase 4 — Scale & Polish (v0.6–v0.9) ✅

Provider catalog, fast-path HTTP, skills, voice, bypass mode, i18n completion.

**Status:** Released as `Aegis AI v0.9`.

- [x] Unified model catalog (10,978 models, 119 providers)
- [x] Fast-path HTTP (LRU cache + dedup, <400ms warm latency)
- [x] 15 builtin Skills
- [x] Voice I/O (push-to-talk, Whisper STT, OS-native/ElevenLabs TTS)
- [x] Bypass Mode (user-controlled, hard-deny list always enforced)
- [x] +14 tools (total 28): git, process, code_eval, memory, skill_set, …
- [x] CalDAV calendar integration (with correct RFC 5545 line unfolding in v1.0)
- [x] 7-locale i18n (EN, VI, ES, FR, DE, JA, ZH-CN)
- [x] Bug sweep: Actions workflow, locale expansion, boot panic, quarantine panel, XSS
- [x] Documentation rewrite for professional presentation

---

## v1.0.0 — General Availability ✅

Comprehensive bug-fix & polish sweep: 18 hard compile errors fixed across
the Rust backend, 30+ clippy warnings cleaned up, 4 failing unit tests
fixed (104/104 now pass), GitHub Actions workflows corrected, version
bumped to 1.0.0 across all manifests. See CHANGELOG.md for the full list.

---

## v1.1.0 — Toolchain & Dependency Modernization ✅

Full-stack upgrade on Rust 1.97.1: Rust edition 2024, all backend crates
moved to current majors (Tauri 2.11, reqwest 0.13, rusqlite 0.40, keyring 4,
enigo 0.6, screenshots 0.8, notify 8, windows 0.62, …), frontend moved to
React 19 + Vite 8 (Rolldown) + Tailwind CSS 4 + zustand 5, ESLint 10 flat
config introduced, CI bumped to Node 22. 104/104 unit tests pass, clippy
`-D warnings` clean. See CHANGELOG.md for the full list.

---

## Next: v1.2 — Hardening & macOS

| Task | Priority |
|---|---|
| SQLCipher encryption by default | High |
| Neural embeddings (all-MiniLM-L6-v2) | High |
| Signed confirmation tokens | High |
| OS keychain for all credentials | High |
| AWS Bedrock SigV4 signing (currently stubbed) | High |
| Credential zeroization (`zeroize` crate) | Medium |
| Auto-update with signed manifest | Medium |
| `aegis export` / `aegis forget` CLI commands | Medium |
| macOS support | Medium |
| Fuzz targets for IPC parser + safety evaluator | Low |
| Reproducible builds + binary signing | Low |

