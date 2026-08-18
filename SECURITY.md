# Aegis AI — Security Policy

**Last updated:** 2026-08-19

---

## Supported Versions

| Version | Supported |
|---|---|
| 0.9.x | ✅ |
| < 0.9 | ❌ |

Only the latest release receives security updates.

---

## Reporting a Vulnerability

**Do not open a public GitHub issue.** Instead:

1. Email the maintainer with a description, steps to reproduce, and proof-of-concept.
2. You will receive acknowledgment within 48 hours.
3. We will triage, develop a fix, and coordinate a disclosure timeline.

**We commit to:**

- Crediting reporters in release notes (unless anonymous preference stated)
- Patching Critical-severity issues within 48 hours, High within 14 days
- Notifying users of any vulnerability that may have exposed data

---

## Threat Model Summary

| Threat | Mitigation |
|---|---|
| **AI prompt injection** | 5-level risk classifier + hard-deny list; every computer-use action gated |
| **Credential theft** | OS keychain (`keyring` crate); `config.toml` permissions `0600` on Unix |
| **Malicious local process** | Process monitor (15s polling) → threat signatures → quarantine + kill |
| **Supply chain attack** | `Cargo.lock` + `package-lock.json` checked in; `cargo audit` in CI |
| **Auto-defense false positive** | Medium severity = notify only; Critical = unambiguous; all actions audited + reversible |

---

## Security Features

- **Continuous monitoring** — runs in both Continuous and On-demand modes
- **Quarantine** — copy-then-delete; files can be restored or permanently deleted
- **Audit log** — every computer-use action and defensive action recorded in SQLite

---

## Hardening Recommendations

1. Use a **local provider** (Ollama, LM Studio) when possible to eliminate cloud data exposure
2. Keep `auto_defense` enabled
3. Review the activity log periodically
4. Do not enable `allow_autonomous` unless you fully trust the active provider
5. Restrict `write_path_whitelist` to directories you're comfortable with the AI modifying
