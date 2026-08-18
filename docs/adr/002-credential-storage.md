# ADR 002: Credential Storage

## Status

Accepted

## Context

Aegis AI connects to 90+ external AI provider APIs, each requiring an API
key or other credentials. These credentials are high-value targets — a
stolen API key can be used to make unauthorized requests, incur costs, or
access private data. The credential storage mechanism must satisfy:

- **Confidentiality** — API keys must not be readable by other users or
  processes on the same machine.
- **No plaintext on disk** — Keys must never appear in configuration files,
  log files, or any world-readable file.
- **Cross-platform** — The mechanism must work on Linux, macOS, and Windows.
- **User convenience** — Users should enter a key once, not on every request.
- **Key rotation** — Users must be able to update or delete keys at any time.

Options considered:

1. **Plaintext in config.toml** — Simplest approach. Keys stored alongside
   other configuration. Convenient but completely insecure — any process
   with filesystem access can read the keys.
2. **Environment variables** — Keys read from `OPENAI_API_KEY` etc. at
   startup. Better than plaintext files (not committed to git), but still
   visible in `/proc/<pid>/environ` on Linux and in process listing on
   Windows. Not practical for 90+ providers.
3. **Encrypted file** — Application-managed encrypted file using a
   master password. Requires the user to enter a password on each startup.
   Adds complexity (encryption, key derivation, file format).
4. **OS keychain via `keyring` crate** — Delegate to the platform's
   native credential storage (macOS Keychain, Linux Secret Service,
   Windows Credential Manager). Keys are encrypted by the OS and
   retrieved on demand.

## Decision

We use **OS keychain storage via the `keyring` crate**.

Rationale:

- The OS keychain is the standard credential storage mechanism on each
  platform. It is maintained and audited by the OS vendor.
- Keys are encrypted at rest by the OS and never appear in plaintext on
  disk (not in config files, not in environment variables).
- The `keyring` crate provides a uniform API across macOS, Linux, and
  Windows with zero additional infrastructure.
- Keys are retrieved on demand (per-request) and go out of scope
  immediately after use, minimizing the window for memory scraping.
- Key rotation is trivial — `keyring::Entry::set_password()` overwrites
  the existing entry.

## Consequences

### Positive

- Credentials are encrypted by the OS vendor's battle-tested implementation.
- No master password required — the keychain is unlocked by the user's
  OS login.
- Uniform API across all platforms via `keyring`.
- Zero credential leakage to configuration files or logs.
- Simple key rotation and deletion.

### Negative

- **Linux dependency on Secret Service** — The `keyring` crate requires
  a D-Bus Secret Service implementation (GNOME Keyring, KDE Wallet, or
  `pass`). Headless Linux environments without a keyring daemon will
  fail. This is documented in the troubleshooting guide.
- **Keychain not available in CI** — Automated tests that require
  provider credentials must either mock the keyring or use environment
  variables as a fallback.
- **Memory lifetime** — The API key exists in Rust's heap memory for
  the duration of each HTTP request. A sophisticated local attacker with
  `ptrace` access could read it from process memory. Recommendation for
  v1.0: use the `zeroize` crate to explicitly clear the key after use.

### Risks

- If the OS keychain is compromised (e.g., the user's OS password is
  stolen), all stored API keys are exposed. This is a platform-level
  risk, not an application-level one.
- The `keyring` crate is a dependency that must be audited. It delegates
  to platform libraries, so the attack surface is primarily in the
  Rust-to-FFI binding layer.
