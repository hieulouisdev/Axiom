# Operational Modes

Two modes control how and when the AI is active.

## On-Demand (Default)

AI **dormant until you invoke it**. Security monitor still runs. Minimal resources when idle. Recommended for most users.

## Continuous

AI **always on** with 60-second heartbeat. Responds to: file changes, security events, calendar events, clipboard changes. More resource usage (AI tokens, network, CPU).

### Continuous Config

```toml
[continuous]
heartbeat_interval_secs = 60
watch_filesystem = true
watch_clipboard = true
watch_calendar = true
watch_security = true
```

## Switching

**Modes** in sidebar or **Settings** → **Mode**. Security monitor runs in both modes. Voice I/O works in both.

| Indicator | Mode |
|---|---|
| 🟢 On-Demand | AI dormant, security active |
| 🔵 Continuous | AI active, monitoring events |
