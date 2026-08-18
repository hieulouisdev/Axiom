# Aegis AI — Threat Model

**Version:** v0.9 | **Updated:** 2026-08

---

## 1. System Overview

Tauri 2.0 desktop app (Rust backend + React frontend) connecting to 90+ external AI APIs, storing data in local SQLite, optionally performing computer-use automation.

### Trust Boundaries

| ID | Boundary | Trusted | Untrusted |
|---|---|---|---|
| TB-1 | IPC bridge | Rust backend | React frontend (webview) |
| TB-2 | Disk I/O | Rust process | Filesystem, SQLite |
| TB-3 | Network | Rust TLS client | AI provider APIs |
| TB-4 | OS keychain | `keyring` crate | Keychain daemon |
| TB-5 | Process enumeration | Security monitor | Running OS processes |
| TB-6 | Shell execution | `computer::exec_command` | Arbitrary shell commands |

---

## 2. Assets

| Asset | Location | Sensitivity | Protection |
|---|---|---|---|
| API keys | OS keychain | Critical | Encrypted by OS, never plaintext on disk |
| Conversations | SQLite | High | Local only; opt-in SQLCipher |
| Knowledge base | SQLite + embeddings | High | Local only; vector BLOBs |
| Audit log | SQLite | Medium | Append-only, tamper-evident |
| Config | `config.toml` | Medium | Filesystem permissions; no secrets |
| Quarantined files | `quarantine/` | Low | Copy-then-delete; user-controlled |

---

## 3. Threat Actors & Mitigations

| Threat Actor | Capability | Key Mitigations |
|---|---|---|
| **Malicious AI provider** | Inject text into responses | Hard-deny list, kill switch, rate limiter, audit log |
| **Local malware** | Read memory, scan filesystem, hook syscalls | Security monitor, auto-defense, OS keychain, integrity monitor |
| **Supply chain attacker** | Compromise a dependency | `cargo audit` in CI, lockfiles, CSP, minimal Tauri permissions |
| **Insider / curious dev** | Submit data-exfiltrating PR | Code review, network anomaly detection, audit log |
| **Compromised frontend (XSS)** | Execute JS in webview | CSP, React JSX escaping, safety policy on all computer-use |

---

## 4. Attack Trees

### Credential Theft

```
Goal: Steal API key
├── Read OS keychain → requires root/admin
├── MITM TLS 1.3 → computationally infeasible with rustls
├── Read config.toml → keys never stored there
└── Phish via AI response → social engineering (out of scope)
```

**Residual risk:** Memory-scraping by local malware with root privileges. Mitigated by short credential lifetime in memory.

### Prompt Injection

```
Goal: Cause AI to execute unintended action
├── Direct injection → hard-deny list + safety confirmation
├── Indirect via web content → content sanitization + source marking
├── Via knowledge base (RAG) → source attribution in results
└── Via AI provider response → rendered as text, safety policy still gates actions
```

**Residual risk:** Multi-turn social engineering via AI responses.

### Data Exfiltration

```
Goal: Extract user data
├── Read aegis.db → mitigated by SQLCipher opt-in
├── Side-channel via AI provider → data sent to provider by design (use local providers)
├── DNS exfiltration → network monitor detects unusual traffic
└── Clipboard → audit log records all clipboard writes
```

---

## 5. Mitigations Summary

| Mitigation | Threats Addressed |
|---|---|
| 5-level risk classifier + hard-deny list | Prompt injection, IPC abuse |
| OS keychain storage | Credential theft from disk |
| Kill switch + rate limiter | Runaway agent, rapid-fire attacks |
| Audit log | Forensics, insider detection |
| Auto-defense (monitor + defender) | Local malware, miners, reverse shells |
| Quarantine + integrity monitor | Tampering, supply chain |
| CSP + Tauri capabilities | XSS, IPC abuse from frontend |
| TLS 1.3 (rustls) | MITM, credential interception |

---

## 6. Residual Risks

| Risk | Severity | Likelihood |
|---|---|---|
| Memory scraping of API keys | High | Low |
| Zero-day in Chromium/WebKit | Critical | Low |
| AI provider data harvesting | High | Medium |
| Sophisticated social engineering via AI | Medium | Medium |
| SQLite read by local malware | Medium | Medium |

---

## 7. v1.0 Recommendations

1. SQLCipher by default
2. Signed confirmation tokens (HMAC, 60s expiry)
3. Credential zeroization (`zeroize` crate)
4. Prompt provenance tracking
5. Outbound network allowlist
6. Process memory protection (`prctl(PR_SET_DUMPABLE, 0)`)
7. Fuzz targets for IPC parser + safety evaluator
8. Chained-hash audit log integrity
9. Dependency pinning + reproducible builds
