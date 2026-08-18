# Aegis AI — Security White Paper

**Version:** v0.7 (Phase 4)  
**Date:** 2025-07  
**Audience:** Security engineers, compliance officers, enterprise evaluators

---

## 1. Executive Summary

Aegis AI is a cross-platform desktop AI assistant built on Tauri 2.0 (Rust +
React). It supports 90+ AI providers, 28 agent tools, voice I/O, security
monitoring, and retrieval-augmented generation — all while maintaining a
security-first architecture that prioritizes user data sovereignty and
defense-in-depth.

This white paper describes the security properties of the Aegis AI
architecture, covering process isolation, credential management, data
protection at rest and in transit, AI safety controls, automated defense,
privacy guarantees, and compliance readiness.

**Key security claims:**

- Credentials are stored exclusively in the OS keychain — never on disk in
  plaintext, never in configuration files.
- All external network communication uses TLS 1.3 via rustls with no HTTP
  fallback.
- Every computer-use action passes through a 5-level risk classifier with a
  hard-deny list that cannot be overridden, even by the user.
- The application sends zero telemetry by default. All AI interactions stay
  between the user and their chosen provider.
- An audit log records every significant action for forensic analysis.

---

## 2. Architecture Security Properties

### 2.1 Process Isolation

Aegis AI runs as a single OS process with two logical compartments:

| Compartment | Runtime | Privilege Level |
|---|---|---|
| Rust backend | Native binary | Full process access (file I/O, network, shell) |
| React frontend | Tauri webview (WRY/WebKit) | Sandboxed rendering, no direct OS access |

The frontend cannot access the filesystem, network, or shell directly. It
communicates with the backend exclusively through the Tauri `invoke_handler`
IPC bridge, which serializes all arguments and return values over a
structured channel.

### 2.2 IPC Sandboxing

Tauri 2.0 introduces a **capability-based permission system**. Each Tauri
command is registered in `invoke_handler![]` and the set of allowed
commands is defined in `src-tauri/capabilities/default.json`. Commands not
listed in the capability file are inaccessible from the frontend, even if
they are registered in Rust.

This means:

- The frontend cannot call arbitrary Rust functions.
- New commands must be explicitly added to the capability file.
- Plugin permissions are scoped per-plugin, not blanket.

### 2.3 Content Security Policy

The Tauri webview enforces a Content Security Policy that restricts:

- **Script sources:** Only `self` (inline and bundled scripts). No remote
  scripts, no `eval()`, no `unsafe-inline` in production builds.
- **Style sources:** `self` and `unsafe-inline` (required by Tailwind CSS).
- **Connect sources:** Only `ipc://localhost` (Tauri IPC). No direct
  `fetch()` to external URLs from the frontend.
- **Image sources:** `self` and `data:` (for base64 screenshots).

All external network requests are made from the Rust backend, not from the
webview.

---

## 3. Credential Management

### 3.1 Storage: OS Keychain

API keys and provider credentials are stored using the `keyring` crate,
which delegates to the platform's native credential storage:

| Platform | Backend | Encryption |
|---|---|---|
| macOS | Keychain Services | AES-256 (Keychain manages) |
| Linux | Secret Service API (GNOME Keyring / KDE Wallet) | DBus-encrypted |
| Windows | Credential Manager | DPAPI (user-level encryption) |

### 3.2 Never Plaintext

Credentials are **never** written to `config.toml`, `aegis.db`, log files,
or any other plaintext file. The `config.toml` stores only non-sensitive
provider configuration (base URL, model name, feature flags). The actual
API key is retrieved from the keychain at request time and held in memory
only for the duration of the HTTP call.

### 3.3 Credential Lifecycle

```
User enters API key
    → keyring::Entry::set_password(key)  [OS keychain]
    → key dropped from Rust stack

AI request starts
    → key = keyring::Entry::get_password()  [retrieved from keychain]
    → HTTP request with Authorization header
    → key variable goes out of scope
    → [Recommendation v1.0: explicit zeroize()]
```

### 3.4 Key Rotation

Users can update a provider's API key at any time via the Settings UI. The
new key replaces the old one in the OS keychain — there is no history of
previous keys in the keychain entry.

---

## 4. Data at Rest

### 4.1 SQLite Database

All persistent user data is stored in a single SQLite database file
(`aegis.db`) located in the platform's application data directory:

| Platform | Path |
|---|---|
| Linux | `~/.local/share/aegis-ai/aegis.db` |
| macOS | `~/Library/Application Support/aegis-ai/aegis.db` |
| Windows | `%APPDATA%\aegis-ai\aegis.db` |

The database is opened with the following SQLite pragmas for safety:

```sql
PRAGMA journal_mode = WAL;       -- Concurrent read/write
PRAGMA foreign_keys = ON;        -- Enforce referential integrity
PRAGMA busy_timeout = 5000;      -- Retry on lock contention
```

### 4.2 SQLCipher Opt-In

Users can enable SQLCipher encryption for the database at rest. When
enabled, the database is encrypted with AES-256-CTR and HMAC-SHA512 for
integrity. The encryption key is derived from a user-provided passphrase
via PBKDF2 with 256,000 iterations.

This is an opt-in feature in v0.7. **Recommendation for v1.0:** enable
SQLCipher by default with a key derived from the OS keychain.

### 4.3 Integrity Monitoring

The `security/integrity.rs` module maintains SHA-256 baselines of critical
application files (configuration, database, quarantine directory). On each
integrity check, current hashes are compared against the baseline and
deviations are reported as security alerts.

### 4.4 Quarantine

The `security/quarantine.rs` module implements a copy-then-delete quarantine
mechanism. Suspected malware files are:

1. Copied to `~/.local/share/aegis-ai/quarantine/<sha256>` with metadata.
2. The original file is deleted (not just renamed).
3. The user can review, restore, or permanently delete quarantined files.

---

## 5. Data in Transit

### 5.1 TLS 1.3 via rustls

All outbound HTTP requests use `reqwest` configured with `rustls` as the
TLS backend. **There is no HTTP fallback.** If a provider's endpoint does
not support HTTPS, the request fails.

Properties:
- **TLS 1.3 preferred** — falls back to TLS 1.2 only if the server does
  not support 1.3.
- **No system TLS** — rustls is statically linked, ensuring the same TLS
  implementation across all platforms regardless of OS TLS version.
- **Certificate verification** — rustls uses the Mozilla root certificate
  bundle. Custom CA certificates are not supported (by design, to prevent
  MITM via installed root CAs).

### 5.2 Request Isolation

Each AI provider request is an independent HTTP call. There are no
persistent connections, session cookies, or server-side state between
requests. The `Authorization` header is set per-request and not stored in
any HTTP client state.

### 5.3 Streaming Security

Streaming AI responses are delivered via Tauri events (not HTTP SSE from
the frontend). The Rust backend reads the SSE stream from the provider and
emits individual `ai://chunk` events to the frontend over the IPC bridge.
The frontend never has a direct network connection to the AI provider.

---

## 6. AI Safety

### 6.1 Five-Level Risk Classification

Every proposed computer-use action is classified before execution:

| Level | Behavior | Examples |
|---|---|---|
| **Safe** | Execute immediately | `ls`, `cat`, `pwd`, `git status` |
| **Low** | Execute immediately (minor side effects) | Write to whitelisted directory, open trusted app |
| **Medium** | Require user confirmation | Write to non-whitelisted path, run non-whitelisted command |
| **High** | Require user confirmation | Delete files, system-level changes, network-elevated ops |
| **Critical** | **Always** require confirmation, even in autonomous mode | Disk format, kernel changes, privilege escalation |

### 6.2 Hard-Deny List

A set of actions that are **blocked unconditionally**, regardless of user
confirmation or bypass mode:

- Writing to system paths (`/etc/`, `/usr/`, `C:\Windows\`, etc.)
- Destructive commands (`rm -rf /`, `mkfs`, `dd if=`, `format`, `:(){:|:&};:`)
- Credential dumpers (`mimikatz`, `procdump`, `lsass`)
- Reverse shells (`/dev/tcp/`, `nc -e`, `bash -i >&`)
- Privilege escalation (`sudo su`, `runas /user:admin`)

The hard-deny list is enforced in `computer/safety.rs` and cannot be
disabled by any configuration setting.

### 6.3 Kill Switch

A global `AtomicBool` that, when tripped, causes all running agent loops
to abort on their next iteration. The frontend exposes a prominent "STOP"
button that trips the kill switch. Once tripped, it stays tripped until
the user explicitly resets it — preventing the AI from restarting itself.

### 6.4 Rate Limiter

The `computer/rate_limiter.rs` module limits the frequency of computer-use
actions to prevent rapid-fire attacks (e.g., an AI agent attempting to
execute hundreds of shell commands per second). Default: 10 actions per
minute, configurable by the user.

### 6.5 Audit Log

Every significant action is recorded in the `activities` SQLite table:

| Field | Description |
|---|---|
| `kind` | Action category (e.g., `chat.user`, `computer.exec`, `security.quarantine`) |
| `summary` | Human-readable description |
| `risk` | Risk level (if applicable) |
| `created_at_ms` | Unix timestamp |

The audit log is append-only and can be exported (v0.6+) for external
forensic analysis.

---

## 7. Auto-Defense

### 7.1 Threat Signatures

The security monitor (`security/monitor.rs`) polls running processes every
15 seconds and matches their command lines against a configurable list of
threat signatures. Default signatures cover:

- Reverse shells (`/dev/tcp/`, `nc -e`, `bash -i`)
- Credential dumpers (`mimikatz`, `procdump`, `lsass`)
- Crypto miners (`xmrig`, `stratum+tcp`, `minerd`)
- Port scanners (`nmap`, `masscan`, `zmap`)

Users can add custom signatures via `config.toml`.

### 7.2 Escalation Ladder

The `security/defender.rs` module consumes threat detections and escalates
response based on severity:

1. **Notify** — Emit `security://threat` event, show toast in UI.
2. **Quarantine** — Copy-then-delete the offending binary (severity ≥ Medium).
3. **Kill** — Terminate the offending process (severity = Critical).

Each escalation step is logged to the audit log.

### 7.3 File Scanner

The `security/scanner.rs` module performs on-demand file hash scanning
against a built-in list of known-bad SHA-256 hashes (EICAR test file +
sample signatures). Users can trigger scans from the Security UI.

### 7.4 Network Anomaly Detection

The `security/network.rs` module monitors outbound network connections for
anomalous patterns:

- Connections to known C2 infrastructure.
- Unusual outbound ports (not 443, 80).
- High-volume data transfers to unrecognized hosts.

On Linux, this uses `procfs` to enumerate `/proc/net/tcp` and
`/proc/net/tcp6`. On Windows, it uses `GetExtendedTcpTable`.

---

## 8. Privacy

### 8.1 No Telemetry by Default

Aegis AI collects **zero telemetry** by default. There is no analytics SDK,
no crash reporter, no usage tracking. The application does not phone home.

If telemetry is ever added as an opt-in feature, it will be:

- Explicitly opt-in with a clear consent dialog.
- Limited to anonymized, aggregated usage metrics.
- Documented in this white paper and the privacy policy.

### 8.2 GDPR Export / Forget

The `memory_export_all` and `memory_forget_all` commands implement GDPR
data portability and right-to-erasure:

- **Export:** Generates a JSON archive of all user data (conversations,
  knowledge base, activities, configuration).
- **Forget:** Permanently deletes all user data from SQLite and the OS
  keychain. This operation is irreversible.

### 8.3 On-Device Processing

Several features operate entirely on-device without any network
communication:

- Character-trigram embedding generation (no cloud embedding API).
- Safety policy evaluation (all rules are local).
- Security monitoring and threat detection (no cloud lookup).
- Audit logging (all records are local).
- Voice TTS (OS-native synthesis as default; ElevenLabs is opt-in).

### 8.4 Provider Isolation

Each AI provider receives only the messages explicitly sent to it. The
application does not share conversation history, knowledge base contents,
or user configuration across providers. Switching providers starts a fresh
context.

---

## 9. Compliance

### SOC 2 Type II Readiness Checklist

| Control | Status | Evidence |
|---|---|---|
| CC6.1 — Logical access controls | ✅ | OS keychain, CSP, capability-based IPC |
| CC6.2 — Authentication | ✅ | API keys in keychain; no shared credentials |
| CC6.3 — Encryption at rest | ⚠️ Opt-in | SQLCipher available; default v1.0 |
| CC6.6 — Encryption in transit | ✅ | TLS 1.3 (rustls), no HTTP fallback |
| CC7.1 — Security monitoring | ✅ | Process monitor, file integrity, network anomaly |
| CC7.2 — Incident response | ✅ | Auto-defense (notify → quarantine → kill) |
| CC7.3 — Vulnerability remediation | ✅ | `cargo audit` in CI, dep update policy |
| CC8.1 — Change management | ✅ | Code review required, lockfile integrity |
| P1.1 — Privacy notice | ✅ | Privacy policy, no telemetry by default |
| P2.1 — Data retention | ✅ | User-controlled export/forget |
| P3.1 — Consent | ✅ | Explicit opt-in for all data-sharing features |
| P4.1 — Data minimization | ✅ | Only conversation text sent to provider |

Legend: ✅ Implemented | ⚠️ Partial/Optional | ❌ Not yet implemented

---

## 10. Vulnerability Disclosure Policy

### Reporting

We welcome responsible disclosure of security vulnerabilities. Please
report via:

- **Email:** security@aegis-ai.dev (PGP key available on our website)
- **GitHub:** Security advisory on the Aegis AI repository

### Scope

- The Aegis AI desktop application (Rust backend + React frontend).
- The Tauri IPC bridge and capability configuration.
- The safety policy, credential storage, and security monitoring subsystems.

### Out of Scope

- Vulnerabilities in third-party AI provider APIs.
- Vulnerabilities in the OS keychain implementation (report to OS vendor).
- Social engineering attacks against Aegis AI users.

### Response Timeline

| Severity | Acknowledgment | Fix | Disclosure |
|---|---|---|---|
| Critical | 24 hours | 72 hours | 7 days after fix |
| High | 48 hours | 14 days | 14 days after fix |
| Medium | 72 hours | 30 days | 30 days after fix |
| Low | 7 days | Next release | 90 days after fix |

### Credit

We credit researchers in our security advisories and CHANGELOG, unless
they request anonymity.
