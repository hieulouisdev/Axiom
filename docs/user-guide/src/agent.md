# Computer-Use Agent

The computer-use agent allows the AI to interact with your computer on your
behalf — executing commands, reading and writing files, launching applications,
taking screenshots, and performing GUI automation.

## Overview

When you ask the AI to do something that requires computer interaction (e.g.,
"List the files in my Documents folder" or "Open VS Code"), the agent:

1. **Plans** — Decides which tools to use and in what order.
2. **Requests** — Each tool invocation goes through the **safety policy**.
3. **Executes** — If approved, the tool runs and the result is returned to
   the AI for the next step.
4. **Reports** — The final result is shown in the chat.

## Available Tools

| Tool | Description | Risk Level |
|---|---|---|
| `exec_command` | Execute a shell command | Medium–High |
| `file_read` | Read a file's contents | Safe |
| `file_write` | Write content to a file | Medium–High |
| `open_app` | Launch an application | Low |
| `list_apps` | List installed applications | Safe |
| `screenshot` | Capture the screen | Safe |
| `automate` | Perform GUI automation (mouse, keyboard) | Medium |
| `clipboard_read` | Read clipboard contents | Safe |
| `clipboard_write` | Write to clipboard | Low |
| `web_search` | Search the web | Safe |
| `web_fetch` | Fetch and parse a web page | Safe |
| `regex_search` | Search files with regex | Safe |

## Safety Policy

Every tool invocation is evaluated by the safety policy **before execution**.
Actions are classified into five risk levels:

| Level | Behavior |
|---|---|
| **Safe** | Runs immediately — read-only operations, whitelisted commands |
| **Low** | Runs immediately — minor side effects (write to whitelisted dir) |
| **Medium** | **Requires your confirmation** — writes outside whitelist, non-whitelisted commands |
| **High** | **Requires your confirmation** — file deletion, system-level changes |
| **Critical** | **Always requires confirmation** — disk format, kernel changes, privilege escalation |

### Hard-Deny List

Some actions are **blocked unconditionally** and cannot be performed even
with your confirmation:

- Writing to system paths (`/etc/`, `/usr/`, `C:\Windows\`)
- Destructive commands (`rm -rf /`, `mkfs`, `dd if=`)
- Credential dumpers (`mimikatz`, `procdump`)
- Reverse shells (`nc -e`, `bash -i >&`)
- Privilege escalation (`sudo su`, `runas /user:admin`)

### Confirmation Flow

When an action requires confirmation:

1. A dialog appears with a description of the action and its risk level.
2. Click **Confirm** to proceed, or **Deny** to cancel.
3. Denials are logged in the audit log.

## Bypass Mode

**Bypass mode** skips confirmation for Medium and High risk actions, but
**never** for the hard-deny list. This is useful when you trust the AI
and want it to act without interruption.

- **Only you can enable bypass mode** — the AI cannot enable it for itself.
- Enable it in Settings or via the bypass mode toggle in the chat toolbar.
- An orange indicator appears when bypass mode is active.
- Bypass mode also expands the write-path whitelist to include common
  project directories (~/Documents, ~/Projects, ~/src, ~/code, ~/repos).

## Kill Switch

The **STOP** button (red square) in the chat toolbar immediately halts all
running agent loops. Use it if the AI is doing something unexpected or taking
too long. Once tripped, the kill switch stays active until you reset it,
preventing the AI from restarting itself.

## Rate Limiter

The agent is rate-limited to prevent rapid-fire actions (default: 10 actions
per minute). If the rate limit is hit, further actions are queued until the
window resets. You can adjust the rate limit in Settings.

## Audit Log

Every agent action — including denials — is recorded in the audit log
(visible under **Memory → Activities**). Each entry includes:

- The action type and description.
- The risk level.
- The timestamp.
- Whether it was allowed, denied, or bypassed.

You can export the audit log for external review.
