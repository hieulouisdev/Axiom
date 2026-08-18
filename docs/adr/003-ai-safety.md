# ADR 003: AI Safety

**Status:** Accepted

## Context

The computer-use agent can execute commands, read/write files, launch apps, and automate GUI. Need to balance utility (taking actions) with safety (preventing destructive actions).

Options: binary allow/deny, sandboxing (containers), multi-level risk classification, runtime monitoring only.

## Decision

**5-level risk classification with irrevocable hard-deny list.**

| Level | Behavior |
|---|---|
| Safe | Execute immediately |
| Low | Execute immediately (minor side effects) |
| Medium | Require confirmation |
| High | Require confirmation |
| Critical | Always require confirmation, even in autonomous mode |

Hard-deny list blocks: system path writes, destructive commands, credential dumpers, reverse shells, privilege escalation. Cannot be disabled by any configuration.

User-controlled: `command_whitelist`, `write_path_whitelist`, `allow_autonomous`, `bypass_mode`.

## Consequences

**Positive:** Useful actions happen instantly; destructive actions gated; hard-deny provides absolute safety floor; no external infrastructure needed; testable.

**Negative:** Rule-based (not semantic — can't understand intent); confirmation fatigue risk; must update as new dangerous patterns emerge.

**Risk:** Obfuscated commands could bypass pattern matching (mitigated by evaluating expanded command). TOCTOU between safety check and execution (low probability, accepted).
