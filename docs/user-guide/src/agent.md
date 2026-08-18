# Computer-Use Agent

The AI can interact with your computer — execute commands, read/write files, launch apps, take screenshots, perform GUI automation — all gated by the safety policy.

## How It Works

1. **Plan** — AI decides which tools to use
2. **Request** — each invocation goes through safety policy
3. **Execute** — if approved, tool runs; result fed back to AI
4. **Report** — final result shown in chat

## Available Tools (28)

| Tool | Description | Risk |
|---|---|---|
| `exec_command` | Shell command | Medium–High |
| `file_read/write/list` | File I/O | Safe–High |
| `open_app` / `list_apps` | App launch | Low / Safe |
| `screenshot` | Screen capture | Safe |
| `automate` | GUI automation (mouse/keyboard) | Medium |
| `clipboard_read/write` | Clipboard | Safe / Low |
| `git_op` | Git operations | Medium |
| `code_eval` | Run python3/node/bash | High |
| `memory` | Remember/lookup/search | Safe |
| `skill_set/list` | Switch skills | Safe |

## Safety Policy

Every tool invocation evaluated **before execution**:

| Level | Behavior |
|---|---|
| **Safe** / **Low** | Runs immediately |
| **Medium** / **High** | Requires your confirmation |
| **Critical** | Always requires confirmation |

### Hard-Deny List

Blocked unconditionally: system path writes, destructive commands (`rm -rf /`, `mkfs`), credential dumpers, reverse shells, privilege escalation.

## Bypass Mode

Skip confirmation for Medium/High (hard-deny list **always** enforced). Only you can enable it — the AI cannot self-enable. Orange indicator when active.

## Kill Switch

Red **STOP** button immediately halts all agent loops. Stays tripped until you reset it.

## Audit Log

Every action recorded in **Memory → Activities** with action type, risk level, timestamp, and outcome.
