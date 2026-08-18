# Operational Modes

Aegis AI has two operational modes that control how and when the AI is
active.

## On-Demand Mode (Default)

In on-demand mode, the AI is **dormant until you explicitly invoke it**.
This is the default mode and is recommended for most users.

Behavior:

- The AI only responds when you send a message.
- The security monitor still runs in the background.
- No AI tokens are consumed while idle.
- The application uses minimal resources when not chatting.

Use on-demand mode when:

- You want to minimize AI API costs.
- You only need the AI occasionally.
- You prefer explicit control over when the AI is active.

## Continuous Mode

In continuous mode, the AI is **always on** and acts proactively based on
events.

Behavior:

- A **60-second heartbeat** task runs, checking for new events.
- The AI can respond to:
  - File system changes (new files, modified files in watched directories).
  - Security events (threat detections, integrity violations).
  - Calendar events (approaching meetings, tasks).
  - Clipboard changes (if clipboard monitoring is enabled).
- The AI may initiate conversations or notifications without being asked.

Use continuous mode when:

- You want the AI to monitor your environment and act on events.
- You use the security monitoring features extensively.
- You want proactive notifications about threats or schedule changes.

### Resource Usage

Continuous mode uses more resources:

- **AI tokens** — The heartbeat task may make AI requests on each cycle.
- **Network** — AI requests to the configured provider.
- **CPU** — File watcher, process monitor, and heartbeat task.

### Configuration

In continuous mode, you can configure which event sources trigger AI
activity:

```toml
[continuous]
heartbeat_interval_secs = 60
watch_filesystem = true
watch_clipboard = true
watch_calendar = true
watch_security = true
```

## Switching Modes

Switch between modes in the **Modes** section of the sidebar, or in
**Settings** → **Mode**.

- The security monitor runs in both modes — it is not affected by the
  operational mode setting.
- Voice I/O works in both modes (push-to-talk is always available).

## Mode Indicator

The current mode is shown in the status bar:

| Indicator | Mode | Meaning |
|---|---|---|
| 🟢 On-Demand | On-demand | AI dormant, security active |
| 🔵 Continuous | Continuous | AI active, monitoring events |
