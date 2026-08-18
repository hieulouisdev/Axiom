# Introduction

Welcome to **Aegis AI** — a secure, cross-platform AI assistant that runs on
your desktop and gives you full control over your data and AI interactions.

## What is Aegis AI?

Aegis AI is a desktop application built with Tauri 2.0 (Rust + React) that
connects to **90+ AI providers** including OpenAI, Anthropic, Google Gemini,
Ollama, LM Studio, and many more. It features a computer-use agent with 28
tools, voice input/output, security monitoring, and retrieval-augmented
generation — all with a security-first architecture.

## Key Features

- **Multi-provider support** — Connect to 90+ AI providers and switch between
  them instantly. Use cloud providers for power, or local providers (Ollama,
  LM Studio) for privacy.
- **Computer-use agent** — The AI can read files, execute commands, launch
  applications, take screenshots, and perform GUI automation on your behalf,
  with every action gated by a 5-level safety policy.
- **Security monitoring** — Built-in process monitor, file scanner, and
  network anomaly detector that can automatically quarantine threats and kill
  malicious processes.
- **Memory & RAG** — Persistent conversation history, knowledge base with
  vector search, entity extraction, and retrieval-augmented generation.
- **Voice I/O** — Push-to-talk hotkey, cloud speech-to-text (Whisper), and
  text-to-speech (OS-native or ElevenLabs).
- **Privacy first** — Zero telemetry by default. Your data stays on your
  machine. API keys are stored in your OS keychain, never in plaintext.

## Installation

### Download

Download the latest release for your platform from the
[releases page](https://github.com/aegis-ai/axiom/releases).

| Platform | File |
|---|---|
| Linux (x86_64) | `aegis-ai_0.7.0_amd64.AppImage` |
| macOS (Apple Silicon) | `aegis-ai_0.7.0_aarch64.dmg` |
| macOS (Intel) | `aegis-ai_0.7.0_x64.dmg` |
| Windows | `aegis-ai_0.7.0_x64-setup.exe` |

### First Launch

1. Start Aegis AI. The main window opens with the Chat view.
2. Open **Settings** (gear icon in the sidebar) and configure your first
   AI provider.
3. Enter your API key — it will be stored securely in your OS keychain.
4. Start chatting!

## System Requirements

| Requirement | Minimum | Recommended |
|---|---|---|
| OS | Ubuntu 20.04 / macOS 12 / Windows 10 | Latest LTS / Latest |
| RAM | 4 GB | 8 GB |
| Disk | 200 MB (app) + data | 1 GB |
| Network | For cloud providers | Broadband |

Local providers (Ollama, LM Studio) require additional RAM and GPU resources
for model inference.
