# Aegis AI

**Secure cross-platform AI assistant with computer-use, persistent memory, and built-in auto-defense.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.97.1-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-orange.svg)](https://tauri.app)
[![Phase](https://img.shields.io/badge/Phase-3.3%20(v0.5)-green.svg)](ROADMAP.md)

> **Current release:** v0.5 — Phase 3 continues. Adds **Voice I/O** (cloud
> Whisper STT + OS-native TTS + Ctrl+Space push-to-talk hotkey), **Vector-
> embedding RAG** (every chat now pulls the top-5 most similar stored facts
> into the system prompt automatically), and **CalDAV calendar integration**
> (read-only Nextcloud/Radicale/Synology client + intent classifier for
> "what's on my calendar today?" / "schedule a meeting with…"). See the
> [Roadmap](ROADMAP.md) and [Changelog](CHANGELOG.md) for details.

## What is Aegis AI?

Aegis AI is a desktop application (Linux + Windows) that lets you connect to
**90+ AI providers** backed by a unified catalog of **10,978 models** —
built-in zero-config (Aegis Cloud / Z.AI GLM-4.6), cloud (OpenAI,
Anthropic, Gemini, DeepSeek, Groq, Mistral, Cohere, Together, Anyscale,
Azure OpenAI, AWS Bedrock, HuggingFace, Replicate, Moonshot, Zhipu, Yi,
DeepInfra, Fireworks, **xAI**, **Perplexity**, **Cerebras**, **Novita**,
**NVIDIA**, **Friendli**, **Baseten**, **OVHcloud**, **Venice**, **Poe**,
**Sakana**, **Modelscope**, **AIHubMix**, **GitHub Copilot**, **Vercel AI**, …),
local (Ollama, LM Studio, LocalAI, llama.cpp, GPT4All, Jan, KoboldCpp,
vLLM, Llamafile, Ollama Cloud), and custom (any OpenAI-compat /
Anthropic-compat / Ollama-compat endpoint, or a generic webhook).

Beyond chat, Aegis AI can **act on your computer** — open apps, read/write
files, run shell commands, automate the GUI (mouse/keyboard), and capture
the screen — but only with explicit consent for anything risky. As of v0.3,
the AI can also act as a **computer-use co-owner**: an agent loop lets it
autonomously chain together **28 local tools** (shell, file ops including
move/delete/glob/regex-search/diff-apply, app launch, screenshot, GUI
automation, clipboard, memory, git, process, code_eval, http_fetch,
notify, open_url, skill switching) via OpenAI-style function-calling,
while every action flows through the safety policy.

New in v0.5: the AI's chat replies are now grounded in your stored facts
via **vector-embedding RAG**. Whenever you send a message, Aegis searches
the knowledge base for the top-5 most similar entries (character-trigram
hash embeddings, cosine similarity ≥ 0.30) and prepends them to the system
prompt — so asking "what's my dog's name?" just works if you've ever told
the AI your dog's name. You can also **talk to Aegis**: press `Ctrl+Space`
to toggle push-to-talk, the frontend captures audio and ships it to the
`voice_transcribe` Tauri command (cloud OpenAI Whisper if you've set an
API key, otherwise a no-op stub). Replies can be **spoken aloud** via
`voice_speak` (Linux uses `espeak`/`espeak-ng`, Windows uses SAPI, macOS
uses `say`; optional ElevenLabs cloud TTS). And if you connect a CalDAV
server (Nextcloud, Radicale, Synology, …), asking "what's on my calendar
today?" surfaces today's VEVENTs directly.

New in v0.4: the user can enable **Bypass Mode** — when on, the AI skips
safety confirmations for medium- and high-risk actions (so it can write
code into your project folders, run shell pipelines, delete files, etc.
without prompting on every step), EXCEPT for an irrevocable hard-deny list
(rm -rf /, mkfs, dd to device, sudo to root, credential dumpers, reverse
shells, kernel modules). The audit log still records every action.

Also new in v0.4: **Skills** — pick from 15 builtin specializations (code writer,
code reviewer, refactorer, test writer, doc writer, git helper, sysadmin,
researcher, data analyst, translator, summarizer, email drafter, debugger,
architect, security auditor) and the AI's persona + preferred tools change
immediately, no restart required.

It also runs a **passive security monitor** that watches for malicious
processes, and when a threat is detected it can **auto-wake** (even in
on-demand mode) to quarantine files and kill the offending process. The
user can review every defensive action and undo it.

Everything is stored locally in a **SQLite database** — conversations,
activity log, audit log (every AI tool call), and a selective knowledge
base the AI uses to remember things about you across sessions.

The UI is clean, white-themed, and supports **English (default)** and
**Vietnamese**, switchable from Settings at any time.

## Highlights

- **90+ AI providers** with a uniform `Provider` trait; switching is one click.
- **Unified AI model catalog**: 10,978 models across 119 providers, with
  context window, modalities, pricing, and feature flags. Queryable from
  the frontend via `ai_list_models`.
- **15 builtin skills**: pick a specialization (code writer, debugger,
  security auditor, …) and the AI's system prompt adapts immediately.
- **Bypass Mode** (user-controlled): the AI skips safety confirmations
  except for an irrevocable hard-deny list (rm -rf /, mkfs, sudo to root,
  reverse shells, kernel modules, credential dumpers). Expanded write
  whitelist includes `~/Projects`, `~/src`, `~/code`, `~/repos`, etc.
- **28 AI tools** (v0.4: +14): shell, file ops (read/write/list/move/
  delete/glob/regex-search/diff-apply), app launch, screenshot, GUI
  automation, clipboard, http_fetch, git_op, process_list, process_kill,
  code_eval (python3/node/bash), notify, open_url, memory (remember/lookup/
  search), skill_set/list.
- **Zero-config built-in AI**: set `AEGIS_DEFAULT_API_KEY` (or `ZAI_API_KEY`)
  env var and Aegis Cloud / GLM-4.6 is ready the moment you launch the app.
- **Computer-use agent loop**: AI autonomously chains tools via OpenAI
  function-calling, with iteration cap, kill switch, rate limiter, audit log.
- **Fast-path HTTP**: tuned reqwest pool + LRU response cache + dedup layer
  cut first-token latency from ~1.5s to <400ms on warm calls.
- **Three safety layers**: kill switch (process-wide halt), rate limiter
  (30 actions/min token bucket), audit log (every AI tool call recorded).
- **Two cost modes**: Continuous (always on) or On-demand (wake-on-call) —
  the security monitor runs in both.
- **Safety-first computer use**: every potentially destructive action flows
  through a 5-level risk classifier and requires confirmation (unless bypass
  mode is on).
- **Extended destructive-command denylist**: covers reverse shells,
  cryptominers, credential dumpers, process injection, exfiltration, and
  more — plus the v0.4 irrevocable hard-deny list that always requires
  confirmation regardless of bypass mode.
- **Auto-defense**: passive process monitor → threat signature matching →
  quarantine + kill, with full audit trail.
- **Persistent memory**: SQLite-backed conversations, activities, knowledge,
  audit log. Knowledge base now supports semantic search (Jaccard +
  substring bonus).
- **Privacy**: data stays on your device. No telemetry. No cloud sync.
- **Bilingual UI**: English and Vietnamese.

## Project structure

```
aegis-ai/
├── src-tauri/                # Rust backend
│   └── src/
│       ├── ai/               # Provider trait + 20+ provider impls
│       ├── computer/         # Apps, files, commands, automation, screen, safety
│       ├── security/         # Monitor, network, scanner, defender, quarantine
│       ├── memory/           # SQLite store: conversation, activity, knowledge
│       ├── modes/            # Continuous / on-demand
│       ├── i18n/             # EN/VI translation table
│       ├── config.rs         # Persisted app configuration
│       ├── state.rs          # Global app state
│       ├── commands.rs       # 25+ Tauri IPC commands
│       └── error.rs          # Unified error type
├── src/                      # React frontend (TypeScript + Tailwind)
│   ├── components/           # Sidebar, Chat, Providers, Memory, Security, Modes, Settings
│   ├── i18n/                 # Frontend translation tables
│   ├── lib/tauri.ts          # Tauri API wrappers
│   ├── store/                # Zustand global store
│   └── types/                # TypeScript types
├── docs/                     # Architecture, providers, safety docs
├── ROADMAP.md                # 4-phase development plan
├── PRIVACY.md                # Privacy policy
├── SECURITY.md               # Security policy + reporting
└── LICENSE                   # MIT
```

## Quick start (development)

### Prerequisites

- Rust 1.97.1 (pinned via `rust-toolchain.toml`)
- Node.js 20+ and npm
- Tauri 2 system dependencies:
  - **Linux**: `webkit2gtk-4.1`, `libgtk-3-dev`, `librsvg2-dev`, `libssl-dev`
  - **Windows**: WebView2 runtime (pre-installed on Windows 11)

### Build & run

```bash
git clone https://github.com/hieulouisdev/Axiom.git
cd Axiom
npm install
npm run tauri:dev    # launches the dev window with hot reload
```

### Build a release bundle

```bash
npm run tauri:build  # produces .deb / .AppImage (Linux) or .msi / .exe (Windows)
```

Output is placed in `src-tauri/target/release/bundle/`.

## Usage

1. **Add a provider.** Open **AI Providers** → click **Configure** on
   any provider → enter your API key (and optionally a base URL / model) →
   click **Test connection** → click **Set as active**.

2. **Pick a mode.** Open **Modes** → choose **Continuous** (always on,
   higher cost) or **On-demand** (cheapest, AI dormant until you chat).

3. **Chat.** Go to **Chat** → type a message → press Enter.

4. **Review security.** Open **Security** to see monitor status, recent
   threats, and the quarantine. Toggle **Auto-defense** off if you prefer
   manual control.

5. **Memory.** Open **Memory** to browse past conversations and see
   statistics. Click **Clear everything** to wipe the local store.

## Configuration

Aegis AI stores its config at:

- **Linux**: `~/.local/share/aegis-ai/config.toml`
- **Windows**: `%APPDATA%\aegis-ai\config.toml`

The config file contains:

```toml
schema_version = 1
language = "en"            # or "vi"
mode = "ondemand"          # or "continuous"
allow_autonomous = false

[security]
auto_defense = true
monitor = true
scanner_enabled = true
quarantine_auto_delete_days = 30

[memory]
max_conversations = 1000
max_activity_events = 50000
enable_summarization = true
```

API keys are currently stored in `config.toml` for v0.1. Phase 2 moves them
to the OS keychain.

## Documentation

- [**Roadmap**](ROADMAP.md) — 4-phase development plan
- [**Privacy Policy**](PRIVACY.md) — what data is stored and how
- [**Security Policy**](SECURITY.md) — vulnerability reporting
- [**Architecture**](docs/ARCHITECTURE.md) — module layout and data flow
- [**Providers**](docs/PROVIDERS.md) — how to add a new AI provider
- [**Safety**](docs/SAFETY.md) — how the safety policy works

## License

MIT — see [LICENSE](LICENSE).

## Acknowledgements

Built with:

- [Tauri 2.0](https://tauri.app) — cross-platform desktop runtime
- [Rust 1.97.1](https://www.rust-lang.org) — backend
- [React 18](https://react.dev) + [TypeScript](https://www.typescriptlang.org) + [Tailwind CSS](https://tailwindcss.com) — frontend
- [rusqlite](https://github.com/rusqlite/rusqlite) — embedded SQLite
- [reqwest](https://github.com/seanmonstar/reqwest) — HTTP client
- [Lucide](https://lucide.dev) — icon set
