# Settings & Data Privacy

## General

| Setting | Default | Description |
|---|---|---|
| Language | English | 7 languages: EN, VI, ES, FR, DE, JA, ZH-CN |
| Mode | On-Demand | On-Demand or Continuous |
| Theme | System | Light, Dark, System |
| Start on login | Off | Launch at login |
| Minimize to tray | On | Tray instead of quit |

## Safety

| Setting | Default | Description |
|---|---|---|
| Allow autonomous | Off | Skip confirm for Low/Medium |
| Bypass mode | Off | Skip confirm for Medium/High (never hard-deny) |
| Rate limit | 10/min | Max agent actions per minute |

## Security

| Setting | Default | Description |
|---|---|---|
| Auto-defense | Off | Auto quarantine/kill threats |
| Monitoring interval | 15s | Process scan frequency |

## Memory

| Setting | Default | Description |
|---|---|---|
| RAG enabled | On | Include knowledge base in context |
| RAG top-K | 5 | Entries to retrieve |
| Encryption | Off | SQLCipher at rest |

## Data Privacy

- **Zero telemetry** by default
- Data stored locally: `aegis.db` (SQLite), `config.toml` (no secrets), OS keychain (API keys)
- **Export All Data** → JSON archive (GDPR portability)
- **Forget All Data** → irreversible deletion (GDPR right to erasure)

> **Note:** Data sent to AI providers is governed by the provider's privacy policy. Use local providers (Ollama, LM Studio) for sensitive conversations.
