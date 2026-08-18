# Aegis AI — Security White Paper

**Version:** v0.9 | **Updated:** 2026-08 | **Audience:** Security engineers, compliance officers, enterprise evaluators

---

## 1. Executive Summary

Aegis AI is a cross-platform desktop AI assistant built on Tauri 2.0 (Rust + React) supporting 90+ AI providers, 28 agent tools, voice I/O, security monitoring, and RAG — with a security-first architecture.

**Key security claims:**

- Credentials stored exclusively in OS keychain — never in plaintext on disk
- All external communication uses TLS 1.3 via rustls, no HTTP fallback
- Every computer-use action passes through a 5-level risk classifier with irrevocable hard-deny list
- Zero telemetry by default
- Append-only audit log for forensic analysis

---

## 2. Architecture Security

### Process Isolation

| Compartment | Runtime | Privilege |
|---|---|---|
| Rust backend | Native binary | Full process access (file I/O, network, shell) |
| React frontend | Tauri webview (WRY/WebKit) | Sandboxed rendering, no direct OS access |

Frontend communicates exclusively through Tauri `invoke_handler` IPC bridge.

### IPC Sandboxing

Tauri 2.0 capability-based permissions: each command must be explicitly listed in `capabilities/default.json`. Unlisted commands are inaccessible from the frontend.

### Content Security Policy

- **Scripts:** `self` only — no remote scripts, no `eval()` in production
- **Connect:** `ipc://localhost` only — no direct `fetch()` to external URLs
- **Images:** `self` + `data:` (base64 screenshots)

All external network requests originate from the Rust backend, not the webview.

---

## 3. Credential Management

### OS Keychain Storage

| Platform | Backend |
|---|---|
| macOS | Keychain Services (AES-256) |
| Linux | Secret Service API (GNOME Keyring / KDE Wallet) |
| Windows | Credential Manager (DPAPI) |

Credentials **never** written to `config.toml`, `aegis.db`, or log files. Retrieved from keychain per-request, held in memory only for the HTTP call duration.

### Lifecycle

```
User enters key → keyring::Entry::set_password() → key dropped from stack
AI request → key = keyring::Entry::get_password() → HTTP request → key out of scope
```

---

## 4. Data at Rest

### SQLite Database

Opened with WAL mode, foreign keys ON, busy_timeout 5000ms. Located at platform data directory.

### SQLCipher Opt-In

AES-256-CTR + HMAC-SHA512 encryption. Key derived from user passphrase via PBKDF2 (256,000 iterations). **v1.0 recommendation:** enable by default.

### Quarantine

Copy-then-delete mechanism. Suspected malware → copied to `quarantine/<sha256>` → original deleted. User can review, restore, or permanently delete.

---

## 5. Data in Transit

### TLS 1.3 via rustls

- **No HTTP fallback** — non-HTTPS endpoints fail
- **Statically linked** — same TLS implementation across platforms
- **Mozilla root certs** — custom CAs not supported (prevents MITM via installed roots)

### Streaming Security

AI streaming responses delivered via Tauri events (backend → frontend over IPC), not direct frontend HTTP SSE.

---

## 6. AI Safety

### Five-Level Risk Classification

| Level | Behavior |
|---|---|
| Safe | Execute immediately (read-only, whitelisted) |
| Low | Execute immediately (minor side effects) |
| Medium | Require confirmation |
| High | Require confirmation |
| Critical | Always require confirmation, even in autonomous mode |

### Hard-Deny List

Unconditionally blocked: system path writes, destructive commands, credential dumpers, reverse shells, privilege escalation. Cannot be disabled by any configuration.

### Kill Switch

Global `AtomicBool` — trips all agent loops on next iteration. Stays tripped until user resets. AI cannot restart itself.

### Rate Limiter

Default: 10 actions/minute, user-configurable.

---

## 7. Auto-Defense

| Step | Action |
|---|---|
| Monitor | Poll processes every 15s, match threat signatures |
| Notify | Emit `security://threat` event, show UI toast |
| Quarantine | Copy-then-delete binary (severity ≥ Medium) |
| Kill | Terminate process (severity = Critical) |

Default signatures: reverse shells, credential dumpers, crypto miners, port scanners. Users can add custom signatures.

---

## 8. Privacy

- **Zero telemetry** by default — no analytics SDK, no crash reporter
- **GDPR export/forget** — `memory_export_all` and `memory_forget_all` commands
- **On-device processing** — embeddings, safety, security monitoring, audit logging all local
- **Provider isolation** — each provider receives only messages explicitly sent to it

---

## 9. SOC 2 Readiness

| Control | Status |
|---|---|
| CC6.1 — Logical access controls | ✅ OS keychain, CSP, capability-based IPC |
| CC6.2 — Authentication | ✅ API keys in keychain |
| CC6.3 — Encryption at rest | ⚠️ Opt-in SQLCipher |
| CC6.6 — Encryption in transit | ✅ TLS 1.3 (rustls) |
| CC7.1 — Security monitoring | ✅ Process monitor, integrity, network anomaly |
| CC7.2 — Incident response | ✅ Auto-defense |
| P1.1 — Privacy notice | ✅ Privacy policy, no telemetry |
| P2.1 — Data retention | ✅ User-controlled export/forget |

---

## 10. Vulnerability Disclosure

| Severity | Ack | Fix | Disclosure |
|---|---|---|---|
| Critical | 24h | 72h | 7 days after fix |
| High | 48h | 14 days | 14 days after fix |
| Medium | 72h | 30 days | 30 days after fix |
| Low | 7 days | Next release | 90 days after fix |

**Report via:** GitHub Security advisory or email to maintainer. **Do not open public issues.**

**Out of scope:** Third-party provider API vulnerabilities, OS keychain implementation, social engineering.
