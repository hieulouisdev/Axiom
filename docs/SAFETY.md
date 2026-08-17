# Safety policy

This document explains how Aegis AI's safety policy decides which computer-use
actions require explicit user confirmation.

## The principle

The AI is **never** allowed to take a destructive action silently. Every
operation flows through a 5-level risk classifier, and any action at
`Medium` risk or higher requires explicit user consent before it runs.

## Risk levels

| Level | Behavior |
|---|---|
| `Safe` | Action runs immediately. Read-only ops, whitelisted commands. |
| `Low` | Action runs immediately. Writing to user-approved directories, opening trusted apps. |
| `Medium` | **Requires confirmation.** Mutates user files outside the whitelist, runs non-whitelisted commands. |
| `High` | **Requires confirmation.** File deletion, system-level changes, network-elevated ops. |
| `Critical` | **Always requires confirmation**, even with `allow_autonomous=true`. Catastrophic operations: disk format, kernel changes, privilege escalation. |

## Hard-deny list

Some actions are **denied outright** regardless of user confirmation. These
cannot be performed even with `allow_autonomous=true`:

- Writing to system paths: `/etc/`, `/usr/`, `/bin/`, `/boot/`, `/proc/`,
  `/sys/`, `/dev/`, `/root/`, `C:\Windows\`, `C:\Program Files\`, etc.
- Deleting files in system paths.
- Destructive commands: `rm -rf`, `mkfs`, `dd if=`, `format`, `shutdown`,
  `reboot`, `killall`, `:(){:|:&};:`, `>/dev/sda`, `chmod -R 777`,
  `chown -R`, `userdel`, `usermod`, `del /f`, `rd /s`, `reg delete`,
  `regedit`, `net user`, `sc delete`, `systemctl disable`, `apt remove`,
  `apt purge`, etc.

## User-controlled whitelists

The user can configure two whitelists in `config.toml`:

### `command_whitelist`

Shell commands that may be run without confirmation. Defaults:

```toml
command_whitelist = [
  "ls", "cat", "echo", "pwd", "date",
  "git status", "git log",
  "tasklist", "systeminfo",
]
```

The safety policy matches on the first whitespace-separated token, so adding
`"git"` would whitelist `git status`, `git log`, `git diff`, etc. — be
careful.

### `write_path_whitelist`

Directories the AI may write to without confirmation. Defaults:

```toml
write_path_whitelist = ["~/Documents/AegisAI/"]
```

`~` is expanded to the user's home directory.

## The confirmation flow

When an action requires confirmation:

1. The Rust backend returns an `Err(AegisError::SafetyConfirmation { token, summary })`.
2. The React frontend intercepts the error and shows a modal with `summary`
   and a `Confirm` / `Deny` button pair.
3. If the user clicks **Confirm**, the frontend re-issues the original
   request with `authorized=true`. The backend skips the safety policy and
   runs the action.
4. If the user clicks **Deny**, the action is not performed. The denial is
   logged to the activity store.

> **Note (v0.1):** the token-based confirmation table is a stub. The
> frontend re-issues the original command with `authorized=true` rather
> than using the token. Phase 2 will introduce a signed, short-lived
> (60s) token table that prevents replay attacks.

## `allow_autonomous` mode

If the user enables `allow_autonomous = true` in Settings, the safety
policy is bypassed for `Medium` and `Low` risk actions. **`High` and
`Critical` actions still require confirmation.**

This mode is intended for trusted local providers (Ollama, LM Studio)
where the user is comfortable with the AI acting without asking. It is
**off by default** and carries an explicit warning in the UI.

## Activity log

Every action — including denials — is recorded in the `activities` table
with:

- `kind`: action category (e.g. `chat.user`, `computer.exec`, `heartbeat`).
- `summary`: human-readable description.
- `risk`: the risk level (if applicable).
- `created_at_ms`: timestamp.

The log is visible under **Memory → Activities** and can be exported
(Phase 4).

## Testing

The safety policy has unit tests in `src-tauri/src/computer/safety.rs`:

- `rm -rf /tmp/foo` → `RequireConfirmation` ✅
- `ls -la` → `Allow` ✅
- Writing to `/etc/passwd` → `Deny` ✅

Run them with:

```bash
cd src-tauri && cargo test safety
```

## Threat signatures

Separate from the safety policy, the security subsystem matches running
processes against a list of threat signatures (`threat_signatures` in
`config.toml`). Default signatures cover:

- Reverse shell patterns (`/dev/tcp/`, `nc -e`, `bash -i`)
- Credential dumpers (`mimikatz`, `procdump`, `lsass`)
- Crypto miners (`xmrig`, `stratum+tcp`, `minerd`)
- Port scanners (`nmap`, `masscan`, `zmap`)

Users can add custom signatures. See `config.rs::ThreatSignature` for the
schema.
