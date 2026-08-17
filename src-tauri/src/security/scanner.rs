//! On-demand virus scanner.
//!
//! v0.1: file-hash signature matching against a built-in list of known-bad
//! SHA-256 hashes (small sample). Phase 2 integrates:
//! - Linux: ClamAV daemon (`clamdscan`) when available, falls back to local sigs.
//! - Windows: Microsoft Defender API (`MPManager`) when available.

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub path: String,
    pub scanned: bool,
    pub infected: bool,
    pub signature_name: Option<String>,
    pub hash_sha256: String,
    pub size_bytes: u64,
    pub error: Option<String>,
}

/// Sample list of well-known malware SHA-256 hashes. In production this
/// list is loaded from a ClamAV-style daily-update file (Phase 2).
const KNOWN_BAD_HASHES: &[(&str, &str)] = &[
    // EICAR test file
    (
        "275a021bbfb6489e54d471899f7db9d1663fc695ec2fe2dde2c3a259f10e8884",
        "EICAR-Test",
    ),
    (
        "131f95c51cc819465fa1797f6ccacf9d47021c7dc6349c9dc0dd257aded2128e",
        "EICAR-Test-Alt",
    ),
];

/// Scan a single file.
pub fn scan_file(path: &str) -> Result<ScanResult> {
    let p = Path::new(path);
    if !p.exists() {
        return Ok(ScanResult {
            path: path.into(),
            scanned: false,
            infected: false,
            signature_name: None,
            hash_sha256: String::new(),
            size_bytes: 0,
            error: Some("file not found".into()),
        });
    }

    let metadata = std::fs::metadata(p)?;
    let size_bytes = metadata.len();

    let bytes = std::fs::read(p)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash_hex = hex::encode(hasher.finalize());

    let mut result = ScanResult {
        path: path.into(),
        scanned: true,
        infected: false,
        signature_name: None,
        hash_sha256: hash_hex,
        size_bytes,
        error: None,
    };

    for (sig_hash, sig_name) in KNOWN_BAD_HASHES {
        if sig_hash.eq_ignore_ascii_case(&result.hash_sha256) {
            result.infected = true;
            result.signature_name = Some(sig_name.to_string());
            break;
        }
    }

    Ok(result)
}

/// Scan every file under a directory (recursively), up to a max depth.
pub fn scan_directory(path: &str, max_depth: u32) -> Result<Vec<ScanResult>> {
    let mut out = Vec::new();
    walk_dir(Path::new(path), 0, max_depth, &mut out)?;
    Ok(out)
}

fn walk_dir(dir: &Path, depth: u32, max_depth: u32, out: &mut Vec<ScanResult>) -> Result<()> {
    if depth > max_depth {
        return Ok(());
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            walk_dir(&p, depth + 1, max_depth, out)?;
        } else if let Ok(r) = scan_file(&p.to_string_lossy()) {
            out.push(r);
        }
    }
    Ok(())
}
