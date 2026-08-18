//! AI sandbox: enforce file-write allow-lists even in autonomous/bypass mode.
//!
//! Phase 4.2 — prevents the AI agent from writing files outside an
//! explicitly-allowed set of directories. The allow-list is loaded from
//! config and can be expanded at runtime via the Settings UI.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Sandbox policy for AI file writes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxPolicy {
    /// Whether the sandbox is enabled.
    pub enabled: bool,
    /// Allowed write directories (absolute paths).
    pub allowed_dirs: Vec<String>,
    /// Whether the user's home directory subdirs are allowed by default.
    pub allow_home_subdirs: bool,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        let mut dirs = vec!["/tmp".into(), "/var/tmp".into()];
        dirs.extend(Self::platform_defaults());
        Self {
            enabled: true,
            allowed_dirs: dirs,
            allow_home_subdirs: true,
        }
    }
}

impl SandboxPolicy {
    /// Check if a given path is allowed for writing.
    ///
    /// The check works as follows:
    /// 1. If the sandbox is disabled, everything is allowed.
    /// 2. Canonicalize the target path (resolve symlinks, drop `..`).
    /// 3. If any entry in `allowed_dirs` is a prefix of the canonical path,
    ///    the write is allowed.
    /// 4. If `allow_home_subdirs` is true and the path is under the user's
    ///    home directory, the write is allowed.
    /// 5. Otherwise the write is denied.
    pub fn is_write_allowed(&self, path: &Path) -> bool {
        if !self.enabled {
            return true;
        }

        // Resolve the path — if it doesn't exist yet (we're checking a
        // *target* for a write), canonicalize the parent and join the file
        // name. If even the parent doesn't exist, fall back to the raw path.
        let resolved = if path.exists() {
            path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
        } else {
            match path.parent() {
                Some(parent) if parent.exists() => {
                    let canon_parent = parent
                        .canonicalize()
                        .unwrap_or_else(|_| parent.to_path_buf());
                    match path.file_name() {
                        Some(name) => canon_parent.join(name),
                        None => path.to_path_buf(),
                    }
                }
                _ => path.to_path_buf(),
            }
        };

        // Check against explicit allow-list.
        for dir in &self.allowed_dirs {
            let dir_path = PathBuf::from(dir);
            if resolved.starts_with(&dir_path) {
                return true;
            }
        }

        // Check home subdirs if enabled.
        if self.allow_home_subdirs {
            if let Some(home) = dirs::home_dir() {
                if resolved.starts_with(&home) {
                    return true;
                }
            }
        }

        false
    }

    /// Add a directory to the allow-list.
    ///
    /// Normalizes the path to an absolute form. Duplicates are silently
    /// ignored.
    pub fn add_allowed_dir(&mut self, dir: String) {
        let normalized = Self::normalize_dir(&dir);
        if !self.allowed_dirs.contains(&normalized) {
            tracing::info!("sandbox: added allowed dir {}", normalized);
            self.allowed_dirs.push(normalized);
        }
    }

    /// Remove a directory from the allow-list.
    pub fn remove_allowed_dir(&mut self, dir: &str) {
        let normalized = Self::normalize_dir(dir);
        let before = self.allowed_dirs.len();
        self.allowed_dirs.retain(|d| d != &normalized);
        if self.allowed_dirs.len() < before {
            tracing::info!("sandbox: removed allowed dir {}", normalized);
        }
    }

    /// Get the default allowed dirs for the current platform.
    ///
    /// On Linux/macOS: ~/Documents, ~/Downloads, ~/Desktop, ~/Projects
    /// On Windows:     %USERPROFILE%\Documents, %USERPROFILE%\Downloads, %USERPROFILE%\Desktop
    pub fn platform_defaults() -> Vec<String> {
        let home = dirs::home_dir();
        let mut defaults = Vec::new();

        if let Some(home) = home {
            let home_str = home.to_string_lossy().to_string();

            #[cfg(unix)]
            {
                defaults.push(format!("{home_str}/Documents"));
                defaults.push(format!("{home_str}/Downloads"));
                defaults.push(format!("{home_str}/Desktop"));
                defaults.push(format!("{home_str}/Projects"));
            }

            #[cfg(windows)]
            {
                defaults.push(format!("{home_str}\\Documents"));
                defaults.push(format!("{home_str}\\Downloads"));
                defaults.push(format!("{home_str}\\Desktop"));
            }

            #[cfg(not(any(unix, windows)))]
            {
                // Fallback for unknown platforms — just add home.
                defaults.push(home_str);
            }
        }

        defaults
    }

    /// Normalize a directory path string.
    ///
    /// - Expands `~` to the home directory.
    /// - Attempts to canonicalize (resolve symlinks / `..`).
    /// - Falls back to the raw string if resolution fails.
    fn normalize_dir(dir: &str) -> String {
        let expanded = if let Some(rest) = dir.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                home.join(rest).to_string_lossy().to_string()
            } else {
                dir.to_string()
            }
        } else {
            dir.to_string()
        };

        let path = PathBuf::from(&expanded);
        path.canonicalize()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or(expanded)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]
    use super::*;

    #[test]
    fn default_sandbox_is_enabled() {
        let policy = SandboxPolicy::default();
        assert!(policy.enabled);
    }

    #[test]
    fn disabled_sandbox_allows_everything() {
        let mut policy = SandboxPolicy::default();
        policy.enabled = false;
        assert!(policy.is_write_allowed(Path::new("/etc/shadow")));
    }

    #[test]
    fn tmp_is_allowed_by_default() {
        let policy = SandboxPolicy::default();
        assert!(policy.is_write_allowed(Path::new("/tmp/test.txt")));
    }

    #[test]
    fn system_dir_is_blocked() {
        // Create a policy without home_subdirs so the test is deterministic.
        let mut policy = SandboxPolicy::default();
        policy.allow_home_subdirs = false;
        assert!(!policy.is_write_allowed(Path::new("/etc/shadow")));
    }

    #[test]
    fn add_and_remove_dir() {
        let mut policy = SandboxPolicy::default();
        policy.allow_home_subdirs = false;
        policy.allowed_dirs.clear();

        policy.add_allowed_dir("/opt/aegis".into());
        assert!(policy.is_write_allowed(Path::new("/opt/aegis/data.txt")));
        assert!(!policy.is_write_allowed(Path::new("/usr/bin/bad")));

        policy.remove_allowed_dir("/opt/aegis");
        assert!(!policy.is_write_allowed(Path::new("/opt/aegis/data.txt")));
    }

    #[test]
    fn duplicate_add_is_noop() {
        let mut policy = SandboxPolicy::default();
        let before = policy.allowed_dirs.len();
        policy.add_allowed_dir("/tmp".into());
        assert_eq!(policy.allowed_dirs.len(), before);
    }

    #[test]
    fn home_subdirs_allowed_when_flag_set() {
        let policy = SandboxPolicy::default();
        // This test only passes if the runner has a home directory.
        if let Some(home) = dirs::home_dir() {
            let test_path = home.join("some_random_file.txt");
            assert!(policy.is_write_allowed(&test_path));
        }
    }
}
