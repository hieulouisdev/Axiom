# Safety Policy

How Aegis AI's safety policy decides which computer-use actions require confirmation.

---

## The Principle

The AI is **never** allowed to take a destructive action silently. Every operation flows through a 5-level risk classifier. Actions at `Medium` or higher require explicit user consent.

---

## Risk Levels

| Level | Behavior |
|---|---|
| `Safe` | Runs immediately — read-only ops, whitelisted commands |
| `Low` | Runs immediately — write to whitelisted dirs, open trusted apps |
| `Medium` | **Requires confirmation** — write outside whitelist, non-whitelisted command |
| `High` | **Requires confirmation** — file deletion, system-level changes |
| `Critical` | **Always requires confirmation** — disk format, kernel changes, privilege escalation |

---

## Hard-Deny List

Actions **denied outright** regardless of user confirmation or bypass mode:

- **System path writes**: `/etc/`, `/usr/`, `/bin/`, `/boot/`, `C:\Windows\`, etc.
- **Destructive commands**: `rm -rf`, `mkfs`, `dd if=`, `format`, `shutdown`, `:(){:|:&};:`
- **Credential dumpers**: `mimikatz`, `procdump`, `lsass`
- **Reverse shells**: `/dev/tcp/`, `nc -e`, `bash -i >&`
- **Privilege escalation**: `sudo su`, `runas /user:admin`

---

## User-Controlled Whitelists

### `command_whitelist`

```toml
command_whitelist = ["ls", "cat", "echo", "pwd", "date", "git status", "git log"]
```

Matches on the first whitespace-separated token. Adding `"git"` whitelists all git subcommands.

### `write_path_whitelist`

```toml
write_path_whitelist = ["~/Documents/AegisAI/", "~/Projects/", "~/src/", "~/code/"]
```

`~` expands to the user's home directory.

---

## Confirmation Flow

1. Backend returns `Err(AegisError::SafetyConfirmation { token, summary })`
2. Frontend shows modal with `summary` + Confirm/Deny buttons
3. **Confirm** → re-issue request with `authorized=true` → action runs
4. **Deny** → action not performed; denial logged

> **Note:** Token-based confirmation is currently a stub. Phase 2 will introduce signed, short-lived (60s) tokens.

---

## Bypass Mode

When enabled (user-only, AI cannot self-enable):

- Skips confirmation for `Medium` and `High` risk actions
- **Never** skips `Critical` or hard-deny list
- Expands write-path whitelist to include project directories
- Orange indicator shown when active

---

## `allow_autonomous` Mode

Bypasses safety for `Medium` and `Low` actions only. `High` and `Critical` still require confirmation. Off by default with explicit UI warning.

---

## Activity Log

Every action recorded in `activities` table with `kind`, `summary`, `risk`, `created_at_ms`. Visible under **Memory → Activities**.

---

## Threat Signatures

Separate from the safety policy, the security subsystem matches running processes against threat signatures:

- Reverse shells (`/dev/tcp/`, `nc -e`, `bash -i`)
- Credential dumpers (`mimikatz`, `procdump`, `lsass`)
- Crypto miners (`xmrig`, `stratum+tcp`, `minerd`)
- Port scanners (`nmap`, `masscan`, `zmap`)

Users can add custom signatures via `config.toml`.

---

## Testing

```bash
cd src-tauri && cargo test safety
```

Key tests: `rm -rf /tmp/foo` → RequireConfirmation ✅ | `ls -la` → Allow ✅ | write `/etc/passwd` → Deny ✅
