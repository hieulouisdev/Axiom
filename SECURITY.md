# Aegis AI — Security Policy

**Last updated:** 2026-08-17

## Supported versions

Aegis AI is currently pre-1.0. Only the latest release receives security
updates.

| Version | Supported |
|---|---|
| 0.1.x | ✅ |
| < 0.1 | ❌ |

## Reporting a vulnerability

We take security vulnerabilities seriously. If you discover a vulnerability
in Aegis AI, please report it responsibly:

1. **Do not open a public GitHub issue.**
2. Email the maintainer with a description of the issue, steps to reproduce,
   and (if possible) a proof-of-concept.
3. You will receive an acknowledgment within 48 hours.
4. We will work with you to triage the issue, develop a fix, and coordinate
   a disclosure timeline.

We commit to:

- Crediting reporters in release notes (unless they prefer to remain
  anonymous).
- Notifying users of any vulnerability that may have exposed their data,
  with a clear description of the affected versions and mitigations.
- Issuing patched releases within 14 days for High-severity issues, and
  within 48 hours for Critical-severity issues.

## Threat model

Aegis AI is a desktop application that, by design, can execute commands and
modify files on the user's machine. The threats we explicitly defend against
are:

### T1: AI prompt injection

A malicious chat message or web page could trick the AI into issuing a
destructive computer-use action.

**Mitigation:** Every computer-use action passes through the safety policy
(`computer/safety.rs`), which:

- Classifies actions into 5 risk levels (`Safe`, `Low`, `Medium`, `High`,
  `Critical`).
- Hard-denies writes to system paths (`/etc`, `C:\Windows`, etc.) and
  destructive commands (`rm -rf`, `mkfs`, `format`, etc.) regardless of
  AI request.
- Requires explicit user confirmation for any action that is not in the
  user-approved whitelist (`command_whitelist`, `write_path_whitelist`).
- Records every action to the activity log for forensic review.

**Known limitation (v0.1):** the safety confirmation flow is a stub; the
frontend is expected to re-issue the request with `authorized=true` after
the user clicks "Confirm". This will be hardened in Phase 2 with a
short-lived signed token table.

### T2: Credential theft

If an attacker gains read access to `config.toml`, they could steal the
user's AI provider API keys.

**Mitigation (current):** `config.toml` is created with `0600` permissions
on Unix. On Windows, it inherits the user's `%APPDATA%` ACL.

**Mitigation (Phase 2):** Move all API keys into the OS keychain
(`keyring` crate → libsecret on Linux, Credential Manager on Windows).
`config.toml` will only contain non-secret preferences.

### T3: Malicious local process

A malicious process running on the user's machine could attempt to:

- Modify `config.toml` to point the AI at an attacker-controlled endpoint.
- Replace the binary on disk.
- Read the SQLite database to extract conversation history.

**Mitigation:** Aegis AI's security monitor scans running processes every
15 seconds and flags those whose command line matches a known threat
signature. Quarantine + kill is automatic when `auto_defense` is enabled
and the threat severity is `High` or `Critical`.

**Mitigation (Phase 2):** File integrity monitor watches the Aegis AI
binary, `config.toml`, and `~/.bashrc` / Run keys for unauthorized
modifications.

### T4: Supply chain attack

A transitive Rust or npm dependency could be compromised.

**Mitigation:** `Cargo.lock` and `package-lock.json` are checked in.
**Phase 4** will add `cargo supply chain` auditing in CI and pin all
transitive dependencies to specific versions / hashes.

### T5: Auto-defense false positive

A legitimate process (e.g. `find / -name foo`) could be misclassified as
malicious because its command line matches a threat signature pattern.

**Mitigation:**

- Threat signatures default to `Medium` severity (notify only).
- `Critical` severity is reserved for unambiguous patterns (reverse shells,
  credential dumpers, known-miner binaries).
- Every defensive action is logged with a full audit trail and can be
  rolled back from the Security panel (`Restore` for quarantined files).
- `auto_defense` is **on by default** but can be toggled off at any time
  from Settings.

## Security features

### Continuous monitoring

The process monitor runs in **both** Continuous and On-demand modes. Even
when the AI is dormant (to save cost), the monitor keeps polling `/proc`
(Linux) or the ToolHelp snapshot (Windows) every 15 seconds.

### Quarantine

Files flagged as malicious are moved to `<data_dir>/quarantine/` with their
original path recorded. They cannot be executed from that location. They
can be:

- **Restored** to their original location (one-click in the Security panel).
- **Deleted permanently** (irreversible).

### Audit log

Every computer-use action and every defensive action is appended to the
`activities` table in the SQLite database. The log is queryable from the UI
and exportable as CSV / JSON (Phase 4).

## Hardening recommendations for users

1. **Use a local provider when possible** (Ollama, LM Studio, llama.cpp).
   This eliminates the risk of leaking conversation content to a cloud
   endpoint.
2. **Keep `auto_defense` enabled** unless you have a specific reason to
   disable it.
3. **Review the activity log periodically** to catch any unexpected AI
   actions.
4. **Do not enable `allow_autonomous`** unless you fully trust the active
   provider. This setting bypasses safety confirmation.
5. **Restrict the `write_path_whitelist`** in `config.toml` to directories
   you are comfortable with the AI modifying without asking.
