# ADR 002: Credential Storage

**Status:** Accepted

## Context

Aegis AI connects to 90+ AI providers, each requiring API keys. Requirements: confidentiality, no plaintext on disk, cross-platform, user convenience, key rotation.

Options: plaintext config, environment variables, encrypted file, OS keychain via `keyring`.

## Decision

Use **OS keychain storage via `keyring` crate**.

- Encrypted at rest by the OS vendor's implementation
- No master password required (unlocked by OS login)
- Uniform API across macOS, Linux, Windows
- Keys retrieved per-request, go out of scope immediately
- Simple rotation: `keyring::Entry::set_password()` overwrites

## Consequences

**Positive:** Battle-tested OS encryption, no plaintext leakage, simple rotation.

**Negative:** Linux requires Secret Service daemon (GNOME Keyring/KDE Wallet/pass); keychain unavailable in CI (mock or env var fallback); keys exist briefly in Rust heap during requests (recommend `zeroize` for v1.0).

**Risk:** OS keychain compromise (stolen OS password) exposes all keys — platform-level risk, not application-level.
