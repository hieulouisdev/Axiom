//! On-demand virus scanner.
//!
//! Phase 2: Integrates ClamAV daemon when available, falls back to
//! hash-based signature matching. On Windows, adds a stub for
//! Microsoft Defender API.

use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{AegisError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub path: String,
    pub scanned: bool,
    pub infected: bool,
    pub signature_name: Option<String>,
    pub hash_sha256: String,
    pub size_bytes: u64,
    pub error: Option<String>,
    /// Source of the scan result: "clamav", "defender", "hash"
    #[serde(default)]
    pub scanner: String,
}

/// Sample list of well-known malware SHA-256 hashes.
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

/// Check if `clamdscan` is available on PATH.
fn clamav_available() -> bool {
    which_exists("clamdscan")
}

/// Check if a command exists on PATH.
fn which_exists(cmd: &str) -> bool {
    #[cfg(unix)]
    {
        std::process::Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        std::process::Command::new("where")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

/// Scan a single file using ClamAV if available, otherwise hash-based.
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
            scanner: String::new(),
        });
    }

    // Try ClamAV first
    if clamav_available() {
        if let Ok(result) = scan_with_clamav(path) {
            return Ok(result);
        }
    }

    // Try Windows Defender on Windows
    #[cfg(windows)]
    {
        if let Ok(result) = scan_with_defender(path) {
            return Ok(result);
        }
    }

    // Fall back to hash-based scanning
    scan_with_hash(path)
}

/// Scan a file using ClamAV daemon.
pub fn scan_with_clamav(path: &str) -> Result<ScanResult> {
    let output = std::process::Command::new("clamdscan")
        .args(["--multiscan", "--fdpass", path])
        .output()
        .map_err(|e| AegisError::Security(format!("clamdscan: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let metadata = std::fs::metadata(path)?;
    let size_bytes = metadata.len();

    // Compute hash for reference
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash_hex = hex::encode(hasher.finalize());

    // Parse ClamAV output
    // Format: "/path: Trojan.Generic FOUND" or "/path: OK"
    let infected = stdout.contains("FOUND");
    let signature_name = if infected {
        // Try to extract the signature name
        stdout.lines()
            .find_map(|line| {
                if line.contains("FOUND") {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 2 {
                        let sig = parts[1].trim();
                        let sig = sig.strip_suffix("FOUND").unwrap_or(sig).trim();
                        Some(sig.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
    } else {
        None
    };

    Ok(ScanResult {
        path: path.into(),
        scanned: true,
        infected,
        signature_name,
        hash_sha256: hash_hex,
        size_bytes,
        error: None,
        scanner: "clamav".into(),
    })
}

/// Windows: Scan using Microsoft Defender API.
#[cfg(windows)]
fn scan_with_defender(path: &str) -> Result<ScanResult> {
    // Use MpManagerStartScan via the windows crate
    // For now, fall back to calling the Defender CLI
    let output = std::process::Command::new("MpCmdRun")
        .args(["-Scan", "-ScanType", "3", "-File", path])
        .output();

    let metadata = std::fs::metadata(path)?;
    let size_bytes = metadata.len();

    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash_hex = hex::encode(hasher.finalize());

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let infected = stdout.contains("Threat") || stdout.contains("found");
            Ok(ScanResult {
                path: path.into(),
                scanned: true,
                infected,
                signature_name: if infected { Some("WindowsDefender".into()) } else { None },
                hash_sha256: hash_hex,
                size_bytes,
                error: None,
                scanner: "defender".into(),
            })
        }
        Err(_) => {
            // Defender not available
            Err(AegisError::Security("Windows Defender scan not available".into()))
        }
    }
}

/// Hash-based scanning: compute SHA-256 and compare against known-bad hashes.
fn scan_with_hash(path: &str) -> Result<ScanResult> {
    let p = Path::new(path);
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
        scanner: "hash".into(),
    };

    // Check against built-in known-bad hashes
    for (sig_hash, sig_name) in KNOWN_BAD_HASHES {
        if sig_hash.eq_ignore_ascii_case(&result.hash_sha256) {
            result.infected = true;
            result.signature_name = Some(sig_name.to_string());
            break;
        }
    }

    // Try to load additional signatures from ClamAV-style hash files
    if !result.infected {
        if let Ok(extra_sigs) = load_extra_signatures() {
            for (sig_hash, sig_name) in &extra_sigs {
                if sig_hash.eq_ignore_ascii_case(&result.hash_sha256) {
                    result.infected = true;
                    result.signature_name = Some(sig_name.clone());
                    break;
                }
            }
        }
    }

    Ok(result)
}

/// Load additional signatures from hash files in the data directory.
/// Supports ClamAV-style .hdb/.hsb files containing SHA-256 hashes.
fn load_extra_signatures() -> Result<Vec<(String, String)>> {
    let data_dir = crate::config::AppConfig::data_dir();
    let sig_dir = data_dir.join("signatures");
    if !sig_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sigs = Vec::new();
    let entries = std::fs::read_dir(&sig_dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
        if ext == "hdb" || ext == "hsb" || ext == "hash" {
            if let Ok(content) = std::fs::read_to_string(&path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    // Format: hash:size:name  or  hash:name
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 2 {
                        let hash = parts[0].to_string();
                        let name = if parts.len() >= 3 {
                            parts[2].to_string()
                        } else {
                            parts[1].to_string()
                        };
                        sigs.push((hash, name));
                    }
                }
            }
        }
    }

    Ok(sigs)
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
