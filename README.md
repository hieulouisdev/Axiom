<div align="center">
  <img src="public/logoapp.png" alt="Aegis AI Logo" width="150">
</div>

# Aegis AI

**Secure cross-platform AI assistant with computer-use, persistent memory, and built-in auto-defense.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.97.1-orange.svg)](https://www.rust-lang.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-blue.svg)](https://tauri.app)
[![Release](https://img.shields.io/badge/Release-v1.1.0-green.svg)](https://github.com/hieulouisdev/Axiom/releases)
[![GitHub stars](https://img.shields.io/github/stars/hieulouisdev/Axiom?style=social)](https://github.com/hieulouisdev/Axiom)
[![GitHub forks](https://img.shields.io/github/forks/hieulouisdev/Axiom?style=social)](https://github.com/hieulouisdev/Axiom)
</div>

---

## Screenshots

<p align="center">
  <img src="aegis-ai-screenshot-2-guide.png" alt="Aegis AI — Guide View" width="45%" />
  &nbsp;&nbsp;
  <img src="aegis-ai-screenshot-1-chat.png" alt="Aegis AI — Chat View" width="45%" />
</p>

---

## Overview

Aegis AI is a desktop application (Linux + Windows) that connects to **90+ AI providers** with a unified catalog of **10,978 models** — zero-config built-in (Aegis Cloud / Z.AI GLM-4.6), cloud (OpenAI, Anthropic, Gemini, DeepSeek, Groq, Mistral, xAI, Perplexity, Cerebras, NVIDIA, …), local (Ollama, LM Studio, llama.cpp, GPT4All, vLLM, …), and custom endpoints.

Beyond chat, Aegis AI can **act on your computer** — open apps, read/write files, run shell commands, automate the GUI, capture the screen — all gated by a 5-level safety policy with an irrevocable hard-deny list.

---

## Key Features

| Category | Details |
|---|---|
| **90+ Providers** | Uniform `Provider` trait; switch with one click |
| **10,978 Models** | Unified catalog with context window, modalities, pricing |
| **28 Agent Tools** | Shell, file I/O, app launch, screenshot, GUI automation, clipboard, git, http_fetch, code_eval, memory, skill_set, … |
| **15 Builtin Skills** | Code writer, reviewer, debugger, architect, security auditor, sysadmin, researcher, translator, … |
| **RAG / Memory** | Vector-embedding knowledge base, semantic search, persistent SQLite storage |
| **Voice I/O** | Push-to-talk (Ctrl+Space), cloud Whisper STT, OS-native + ElevenLabs TTS |
| **3 Safety Layers** | Kill switch, rate limiter (30/min), audit log — every action recorded |
| **Auto-Defense** | Passive process monitor → threat detection → quarantine + kill |
| **Bypass Mode** | Skip confirmations for Medium/High (hard-deny list always enforced) |
| **7 Languages** | EN, VI, ES, FR, DE, JA, ZH-CN — switchable at any time |
| **Privacy First** | Zero telemetry, no cloud sync, API keys in OS keychain |
| **Fast-Path HTTP** | LRU cache + dedup layer → first-token latency < 400ms on warm calls |

---

## Quick Start

### Prerequisites

- **Rust 1.97.1** (pinned via `rust-toolchain.toml`)
- **Node.js 20+** and npm
- **Tauri 2 system deps**:
  - Linux: `libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libssl-dev patchelf`
  - Windows: WebView2 runtime (pre-installed on Windows 11)

### Build & Run

```bash
git clone https://github.com/hieulouisdev/Axiom.git
cd Axiom
npm install
npm run tauri:dev      # dev window with hot reload
npm run tauri:build    # release bundle → src-tauri/target/release/bundle/
```

---

## Usage

1. **Add a provider** — AI Providers → Configure → enter API key → Test → Set as active
2. **Pick a mode** — Modes → Continuous (always on) or On-demand (cheapest)
3. **Chat** — Type a message → Enter. AI can chain tools via function-calling
4. **Review security** — Security panel: monitor status, threats, quarantine
5. **Memory** — Browse conversations, knowledge base, activity log

---

## Project Structure

```
aegis-ai/
├── src-tauri/src/          # Rust backend
│   ├── ai/                 # Provider trait + 90+ provider impls
│   ├── computer/           # Apps, files, commands, automation, safety
│   ├── security/           # Monitor, network, scanner, defender, quarantine
│   ├── memory/             # SQLite: conversations, activities, knowledge, RAG
│   ├── modes/              # Continuous / on-demand
│   └── i18n/               # 7-locale translation tables
├── src/                    # React frontend (TypeScript + Tailwind)
│   ├── components/         # Chat, Providers, Memory, Security, Modes, Settings
│   └── i18n/               # Frontend translation tables
└── docs/                   # Architecture, providers, safety, ADRs, user guide
```

---

## Configuration

Config stored at `~/.local/share/aegis-ai/config.toml` (Linux) or `%APPDATA%\aegis-ai\config.toml` (Windows):

```toml
schema_version = 1
language = "en"              # en | vi | es | fr | de | ja | zh-CN
mode = "ondemand"            # ondemand | continuous

[security]
auto_defense = true
monitor = true
scanner_enabled = true

[memory]
max_conversations = 1000
enable_summarization = true
```

---

## Documentation

| Document | Description |
|---|---|
| [Roadmap](ROADMAP.md) | 4-phase development plan |
| [Changelog](CHANGELOG.md) | Version history |
| [Contributing](CONTRIBUTING.md) | How to contribute |
| [Privacy Policy](PRIVACY.md) | Data handling & privacy |
| [Security Policy](SECURITY.md) | Vulnerability reporting |
| [Architecture](docs/ARCHITECTURE.md) | Module layout & data flow |
| [Providers Guide](docs/PROVIDERS.md) | Adding a new AI provider |
| [Safety Policy](docs/SAFETY.md) | How safety classification works |
| [Developer Guide](docs/developer-guide.md) | Setup, testing, debugging |
| [Security Whitepaper](docs/security-whitepaper.md) | Full security analysis |
| [Threat Model](docs/threat-model.md) | Attack trees & mitigations |

---

## Built With

- [Tauri 2.0](https://tauri.app) — cross-platform desktop runtime
- [Rust 1.97.1](https://www.rust-lang.org) — backend
- [React 18](https://react.dev) + [TypeScript](https://www.typescriptlang.org) + [Tailwind CSS](https://tailwindcss.com) — frontend
- [rusqlite](https://github.com/rusqlite/rusqlite) — embedded SQLite
- [reqwest](https://github.com/seanmonstar/reqwest) — HTTP client
- [Lucide](https://lucide.dev) — icon set

---

## License

MIT — see [LICENSE](LICENSE).
