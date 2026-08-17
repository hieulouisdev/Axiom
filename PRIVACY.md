# Aegis AI — Privacy Policy

**Last updated:** 2026-08-17

Aegis AI is built privacy-first. This document explains what data the
application collects, where it is stored, and how you can control or delete
it.

## 1. Data stays on your device

Aegis AI is a **local-first** desktop application. By default:

- All chat conversations are stored in a local SQLite database on your
  machine (`aegis.db` in the data directory).
- All activity logs (computer-use actions) are stored in the same database.
- All AI provider credentials (API keys) are stored in `config.toml` in your
  data directory. **Phase 2** will move them into the OS keychain.
- No telemetry, no analytics, no crash reporting is sent to any server
  unless you explicitly opt in (the opt-in feature itself is planned for
  Phase 4 and will be **off by default**).

## 2. What gets sent to AI providers

When you send a chat message, the following is sent to whichever AI
provider you have configured as active:

- The full text of the conversation up to that point
  (including any system prompt Aegis AI prepends).
- The model identifier and parameters (temperature, max tokens, etc.).
- Your API key for authentication (sent only to that provider's endpoint).

**What is NOT sent automatically:**

- Files on your disk (the AI can request to read a file, but only if you
  approve — and even then the file content is sent to the AI only when
  needed to answer your question).
- Your screen contents (the screenshot tool exists but is opt-in per call
  in v0.1; Phase 3 will gate it behind explicit consent for continuous
  screen-reading sessions).
- Your activity log or security events.
- Other AI providers' credentials.

Each AI provider has its own privacy policy. Aegis AI cannot control what
your chosen provider does with the messages you send. If you are concerned,
use a **local provider** (Ollama, LM Studio, llama.cpp, etc.) which runs
entirely on your machine and sends nothing to the cloud.

## 3. What the security subsystem collects

The passive security monitor samples the running processes on your machine
every 15 seconds. It stores:

- Process name, PID, command line, and parent PID.
- Whether the process matched a known threat signature.
- The defensive action taken (notified / quarantined / killed).

This information is stored **only** in your local SQLite database. It is
**never** sent anywhere automatically. You can review it under
**Memory → Activities** and clear it at any time with
**Memory → Clear everything**.

Quarantined files are copied to a `quarantine/` subdirectory of your data
directory. They remain there until you restore or delete them.

## 4. Data location

| OS | Data directory |
|---|---|
| Linux | `~/.local/share/aegis-ai/` |
| Windows | `%APPDATA%\aegis-ai\` |

Contents:

- `config.toml` — application configuration (currently including API keys).
- `aegis.db` — SQLite database (conversations, activities, knowledge, events).
- `quarantine/` — quarantined files.
- `logs/aegis.log` — application log (rotated, max 10 MB total).

## 5. Data export

Aegis AI does not currently have an export command. **Phase 4** will add:

- `aegis export` — produces a ZIP archive of all your data (config, database,
  quarantine) for backup or migration.
- `aegis export --conversations-only` — produces a JSON file of just your
  chat history.

## 6. Data deletion

You can delete data at any time:

- **Per conversation**: in the Memory panel, click the trash icon next to a
  conversation.
- **All conversations and knowledge**: Memory → Clear everything.
- **Everything (factory reset)**: delete the entire data directory above.
  Aegis AI will recreate it with default settings on next launch.

## 7. Auto-update checks

Aegis AI does not check for updates automatically in v0.1. **Phase 4**
will add update checks (opt-in); when enabled, the app will request a
manifest from `https://github.com/hieulouisdev/Axiom/releases/latest`
to determine whether a newer version is available. No user-identifying
information is included in that request.

## 8. Children's privacy

Aegis AI is not directed at children under 13. We do not knowingly collect
personal information from children. If you believe a child has provided us
with personal information, please contact us so we can delete it.

## 9. Changes to this policy

We will update this policy as the application evolves. Material changes will
be noted in the release notes accompanying the new version.

## 10. Contact

For privacy questions or data deletion requests, open an issue at
<https://github.com/hieulouisdev/Axiom/issues> or email the maintainer via
the email listed in your `git log` of the project.
