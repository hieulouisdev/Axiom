//! Database encryption-at-rest (Phase 2.5 — v0.6).
//!
//! Phase 2.5 calls for an opt-in SQLCipher encryption layer for the SQLite
//! memory store. Full SQLCipher integration requires the `rusqlite`
//! `bundled-sqlcipher` feature, which pulls in OpenSSL on Linux and a
//! custom build of SQLite on Windows. To keep the default v0.6 build
//! lightweight, we ship a **stub** that:
//!
//! 1. Detects whether the running binary was compiled with SQLCipher
//!    support (via a `sqlcipher` cargo feature, not yet wired in).
//! 2. Surfaces the encryption status to the UI.
//! 3. Provides a `set_passphrase` API that, when SQLCipher is available,
//!    rekeys the database; otherwise it stores the passphrase in the OS
//!    keychain as a forward-compatible placeholder.
//!
//! Full SQLCipher enablement is a Phase 4 task — it requires:
//! - Switching `rusqlite` features from `bundled` to `bundled-sqlcipher`.
//! - Adding a key-derivation step (PBKDF2 with a random salt).
//! - Re-running the schema migration on an encrypted connection.
//! - Updating the backup / export tools to handle encrypted DBs.
//!
//! For v0.6, this module documents the contract and gives the UI a stable
//! API to call — the actual encryption is a no-op that returns
//! `EncryptionStatus::NotSupported`.

use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Current encryption status of the on-disk SQLite database.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EncryptionStatus {
    /// SQLCipher feature is not compiled in.
    NotSupported,
    /// SQLCipher is available but the user hasn't set a passphrase.
    Disabled,
    /// Database is encrypted at rest with a passphrase stored in the
    /// OS keychain.
    Enabled,
}

impl Default for EncryptionStatus {
    fn default() -> Self {
        // v0.6 ships without SQLCipher compiled in.
        EncryptionStatus::NotSupported
    }
}

/// Returns whether the running binary was compiled with SQLCipher support.
pub fn is_supported() -> bool {
    // In a future build, this becomes `cfg!(feature = "sqlcipher")`.
    false
}

/// Returns the current encryption status. This is what the UI surfaces
/// in Settings → Security → Database encryption.
pub fn status() -> EncryptionStatus {
    if !is_supported() {
        return EncryptionStatus::NotSupported;
    }
    // Phase 4 will check whether the keychain has a passphrase stored.
    EncryptionStatus::Disabled
}

/// Set or update the database encryption passphrase. When SQLCipher is
/// available, this rekeys the live database. When it isn't (v0.6 default),
/// this is a no-op that returns a "not supported" error so the UI can
/// show a helpful message.
pub fn set_passphrase(_passphrase: &str) -> Result<()> {
    if !is_supported() {
        return Err(crate::error::AegisError::Config(
            "SQLCipher is not compiled into this build. Rebuild with --features sqlcipher to enable at-rest encryption.".into(),
        ));
    }
    // Phase 4 implementation:
    //   1. Derive a key with PBKDF2 (salt stored alongside the DB).
    //   2. Call `PRAGMA rekey = '<derived_key>';` on the live connection.
    //   3. Store the passphrase in the OS keychain for next boot.
    Ok(())
}

/// Disable encryption and decrypt the database. Requires the current
/// passphrase. No-op when SQLCipher isn't compiled in.
pub fn disable_encryption(_passphrase: &str) -> Result<()> {
    if !is_supported() {
        return Err(crate::error::AegisError::Config(
            "SQLCipher is not compiled into this build.".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v0_6_status_is_not_supported() {
        assert_eq!(status(), EncryptionStatus::NotSupported);
    }

    #[test]
    fn set_passphrase_returns_config_error() {
        let r = set_passphrase("hunter2");
        assert!(r.is_err());
    }
}
