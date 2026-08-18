# Settings & Data Privacy

The Settings view lets you configure all aspects of Aegis AI.

## General Settings

| Setting | Default | Description |
|---|---|---|
| Language | English | UI language (English, Vietnamese) |
| Mode | On-Demand | Operational mode (On-Demand, Continuous) |
| Theme | System | UI theme (Light, Dark, System) |
| Start on login | Off | Launch Aegis AI when you log in |
| Minimize to tray | On | Minimize to system tray instead of quitting |

## Provider Settings

See [Providers](providers.md) for detailed provider configuration.

## Safety Settings

| Setting | Default | Description |
|---|---|---|
| Allow autonomous | Off | Skip confirmation for Low/Medium actions |
| Bypass mode | Off | Skip confirmation for Medium/High (never hard-deny) |
| Rate limit | 10/min | Maximum agent actions per minute |
| Command whitelist | (see defaults) | Commands that run without confirmation |
| Write path whitelist | ~/Documents/AegisAI/ | Directories the AI may write to freely |

## Security Settings

| Setting | Default | Description |
|---|---|---|
| Auto-defense | Off | Automatically quarantine/kill threats |
| Monitoring interval | 15s | Process scan frequency |
| Threat signatures | (built-in) | Process threat patterns |

## Memory Settings

| Setting | Default | Description |
|---|---|---|
| RAG enabled | On | Include knowledge base in AI context |
| RAG top-K | 5 | Number of knowledge entries to retrieve |
| Encryption | Off | SQLCipher encryption at rest |

## Voice Settings

| Setting | Default | Description |
|---|---|---|
| STT provider | Cloud Whisper | Speech-to-text backend |
| TTS provider | OS-native | Text-to-speech backend |
| TTS voice | Default | Voice for TTS output |
| TTS speed | 1.0× | Speech rate |
| Auto-speak | Off | Automatically read AI responses |
| Push-to-talk key | Ctrl+Alt+V | Global hotkey for voice input |

## Data Privacy

### No Telemetry

Aegis AI sends **zero telemetry** by default. There is no analytics,
no crash reporter, and no usage tracking. Your data stays on your machine.

### Data Storage

All user data is stored locally in:

- `aegis.db` — SQLite database (conversations, knowledge, activities).
- `config.toml` — Application configuration (no secrets).
- OS keychain — API keys and credentials (encrypted by the OS).

### GDPR Export

You can export all your data for portability:

1. Open **Settings** → **Data Privacy**.
2. Click **Export All Data**.
3. A JSON archive is generated containing:
   - All conversations and messages.
   - Knowledge base entries.
   - Activity log.
   - Configuration (with secrets redacted).
4. Save the archive to your chosen location.

### GDPR Forget (Right to Erasure)

You can permanently delete all your data:

1. Open **Settings** → **Data Privacy**.
2. Click **Forget All Data**.
3. A confirmation dialog appears — this action is **irreversible**.
4. If confirmed:
   - The SQLite database is deleted.
   - All OS keychain entries for Aegis AI are removed.
   - The configuration file is reset to defaults.
   - Quarantined files are permanently deleted.

### Provider Data

**Important:** Data sent to AI providers is governed by the provider's
privacy policy, not Aegis AI's. When you send a message, it is transmitted
to your configured AI provider. Aegis AI cannot control what the provider
does with this data.

To minimize data exposure:

- Use **local providers** (Ollama, LM Studio) for sensitive conversations.
- Use providers with strong privacy policies for less sensitive tasks.
- Remember that conversation content, including any personal information,
  is sent to the provider as part of the chat request.
