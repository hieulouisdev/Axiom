# Aegis AI — Privacy Policy

**Last updated:** 2026-08-19

Aegis AI is built privacy-first. This document explains what data is collected, where it is stored, and how you can control or delete it.

---

## 1. Data Stays on Your Device

Aegis AI is a **local-first** desktop application:

- All conversations, activities, and knowledge stored in local SQLite (`aegis.db`)
- API keys stored in OS keychain (never in plaintext files)
- **Zero telemetry, no analytics, no crash reporting** — nothing is sent unless you explicitly opt in
- Auto-update checks (when enabled) include no user-identifying information

## 2. What Gets Sent to AI Providers

When you send a chat message, the following goes to your active AI provider:

- Full conversation history (including system prompt)
- Model identifier and parameters
- Your API key (only to that provider's endpoint)

**What is NOT sent automatically:**

- Files on disk (only if you approve a file-read action)
- Screen contents (screenshot is opt-in per call)
- Activity log or security events
- Other providers' credentials

Each AI provider has its own privacy policy. Use **local providers** (Ollama, LM Studio, llama.cpp) to eliminate cloud exposure entirely.

## 3. Security Subsystem Data

The process monitor samples running processes every 15 seconds and stores:

- Process name, PID, command line, parent PID
- Whether it matched a threat signature
- Defensive action taken (notified / quarantined / killed)

This data is stored **only** in your local SQLite database and **never** sent anywhere automatically.

## 4. Data Location

| OS | Path |
|---|---|
| Linux | `~/.local/share/aegis-ai/` |
| Windows | `%APPDATA%\aegis-ai\` |

Contents: `config.toml` (no secrets), `aegis.db`, `quarantine/`, `logs/aegis.log` (rotated, max 10 MB).

## 5. Data Deletion

- **Per conversation**: trash icon in Memory panel
- **All conversations + knowledge**: Memory → Clear everything
- **Factory reset**: delete the entire data directory — Aegis AI recreates it on next launch

## 6. GDPR Export & Forget

- **Export**: `aegis export` — ZIP archive of all data (Phase 4)
- **Forget**: `aegis export --conversations-only` — JSON of chat history (Phase 4)
- **Right to erasure**: `memory_forget_all` clears all stores and keychain entries (irreversible)

## 7. Children's Privacy

Aegis AI is not directed at children under 13. We do not knowingly collect personal information from children.

## 8. Changes to This Policy

Material changes will be noted in release notes accompanying the new version.

## 9. Contact

For privacy questions or data deletion requests, open an issue at [github.com/hieulouisdev/Axiom/issues](https://github.com/hieulouisdev/Axiom/issues).
