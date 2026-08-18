# ADR 003: AI Safety

## Status

Accepted

## Context

Aegis AI includes a computer-use agent that can execute shell commands,
read and write files, launch applications, and perform GUI automation on
behalf of the user. This creates a fundamental tension: the AI must be
useful (which requires the ability to take actions), but it must also be
safe (which requires preventing destructive or unintended actions).

The challenge is that "safe" vs "dangerous" is context-dependent. Deleting
a file in `/tmp/scratch/` is usually fine; deleting a file in `/etc/` is
almost always catastrophic. Running `git status` is safe; running
`rm -rf /` is destructive. But there are gray areas — is running
`docker system prune` safe? It depends on the user's intent.

Several approaches were considered:

1. **Binary allow/deny** — Either the AI can do everything or nothing.
   Simple but either too restrictive (useless) or too permissive
   (dangerous).
2. **Sandboxing** — Run all commands in a container or sandbox (Docker,
   Bubblewrap, Firejail). Secure but adds significant complexity,
   platform dependency, and breaks many legitimate use cases (access to
   user's projects, installed tools, etc.).
3. **Multi-level risk classification** — Classify each proposed action
   into risk levels, require explicit confirmation for risky actions,
   and maintain a hard-deny list for catastrophic actions. Provides
   granularity without sandboxing overhead.
4. **Runtime monitoring only** — Let the AI do anything, but monitor
   and alert after the fact. Too late for destructive actions.

## Decision

We implement a **5-level risk classification with a hard-deny list**.

Every proposed computer-use action is evaluated by the safety policy
before execution and assigned one of five risk levels:

| Level | Behavior | Examples |
|---|---|---|
| `Safe` | Execute immediately | Read-only operations, whitelisted commands |
| `Low` | Execute immediately (minor side effects) | Write to whitelisted directory, open trusted app |
| `Medium` | Require user confirmation | Write outside whitelist, non-whitelisted command |
| `High` | Require user confirmation | File deletion, system-level changes |
| `Critical` | **Always** require confirmation, even in autonomous mode | Disk format, kernel changes, privilege escalation |

A **hard-deny list** blocks actions that are never allowed, regardless of
user confirmation or bypass mode: writing to system paths, destructive
commands (`rm -rf /`, `mkfs`, `dd`), credential dumpers, reverse shells,
and privilege escalation attempts.

Users can customize the experience via:

- **`command_whitelist`** — Commands that skip confirmation.
- **`write_path_whitelist`** — Directories the AI may write to freely.
- **`allow_autonomous`** — Skip confirmation for Medium and Low actions
  (High and Critical still require it).
- **`bypass_mode`** — User-only toggle that skips confirmation for Medium
  and High, but **never** for the hard-deny list.

Rationale:

- Multi-level classification provides the right granularity — most
  useful actions (reading files, running safe commands) happen instantly,
  while potentially dangerous ones are gated.
- The hard-deny list provides an absolute safety floor that cannot be
  lowered by any configuration.
- User-controlled whitelists allow power users to customize the
  experience without compromising the fundamental safety guarantee.
- This approach requires no external infrastructure (no containers, no
  sandboxing daemons) and works identically on all platforms.

## Consequences

### Positive

- The AI can perform useful actions immediately (Safe, Low) without
  interrupting the user with confirmation dialogs.
- Destructive actions are always gated, preventing accidental damage.
- The hard-deny list is an absolute safety guarantee — even a
  compromised AI provider cannot bypass it.
- User-controlled whitelists accommodate different risk tolerances.
- No external dependencies (no Docker, no sandbox runtime).
- The safety policy is testable — unit tests verify that specific
  commands produce specific decisions.

### Negative

- The classification is rule-based, not semantic. The safety policy
  cannot understand the *intent* of a command, only its textual form.
  A seemingly safe command like `python -c "import os; os.remove('/etc/passwd')"`
  might not be caught if the inner payload isn't matched by the deny list.
- Confirmation fatigue — if too many actions require confirmation, users
  may enable `allow_autonomous` or `bypass_mode`, reducing the safety
  benefit. The default whitelists are designed to minimize this.
- The safety policy must be updated as new dangerous patterns emerge.
  This is a maintenance burden, but a necessary one.

### Risks

- **Obfuscated commands** — An attacker (via prompt injection) could
  encode a destructive command in a way that bypasses the pattern
  matching (base64, hex encoding, environment variable expansion).
  Mitigation: the safety policy evaluates the *expanded* command, not
  the raw input. Future: add semantic analysis of command ASTs.
- **Race condition** — Between the safety check and execution, the
  filesystem state could change (TOCTOU). This is inherent to any
  pre-flight check approach and is accepted as a low-probability risk.
