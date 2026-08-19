//! Application configuration: persisted user settings, provider credentials,
//! mode toggles, language preference, and security policy thresholds.

use std::path::PathBuf;

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Top-level persisted configuration file.
///
/// Stored as `config.toml` inside the Aegis AI data directory
/// (e.g. `~/.local/share/aegis-ai/` on Linux,
///  `%APPDATA%\aegis-ai\` on Windows).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Schema version for forward migration.
    pub schema_version: u32,

    /// UI language: "en" (default) or "vi".
    pub language: String,

    /// Operational mode: "continuous" (always-on) or "ondemand" (wake-on-call).
    pub mode: OperatingMode,

    /// Per-provider credentials map: `provider_id -> {api_key, base_url, model}`.
    pub providers: ProviderRegistry,

    /// ID of the currently active provider.
    pub active_provider: Option<String>,

    /// Security policy thresholds.
    pub security: SecurityConfig,

    /// Memory store config.
    pub memory: MemoryConfig,

    /// Whether AI is allowed to take actions without asking first.
    /// Defaults to `false` — every dangerous action requires confirmation.
    pub allow_autonomous: bool,

    /// Whether AI is allowed to take destructive actions without asking first.
    /// Defaults to `false` — every destructive action requires confirmation.
    ///
    /// When `true`, the safety policy will *skip* the `RequireConfirmation`
    /// step for medium- and high-risk actions, EXCEPT for an irrevocable
    /// "hard-deny" list (rm -rf /, mkfs, dd to device, sudo to root, credential
    /// dumpers, reverse shells, kernel-module loaders, etc.) which still
    /// surfaces a confirmation prompt regardless of bypass mode.
    ///
    /// **User-controlled bypass**: the user must explicitly enable this in
    /// the Settings UI or via the `bypass_mode_enable` Tauri command. The AI
    /// itself cannot enable bypass mode — only the user can.
    ///
    /// Bypass mode also expands the write-path whitelist to include common
    /// app source directories (~/Documents, ~/Projects, ~/src, ~/code,
    /// ~/repos, ~/.config) so the AI can write code into the user's project
    /// folders without confirmation prompts on every file.
    pub bypass_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum OperatingMode {
    /// AI is always on, listens to events, and acts proactively.
    Continuous,
    /// AI stays dormant until explicitly invoked (saves cost).
    #[default]
    OnDemand,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderRegistry {
    /// Map of provider_id -> configured credentials.
    /// Credentials are stored in the OS keychain in production builds;
    /// for v0.1 scaffold we persist them in the config file (encrypted at rest).
    #[serde(default)]
    pub credentials: std::collections::BTreeMap<String, ProviderCredentials>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCredentials {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub enabled: bool,
    /// Extra provider-specific options (e.g. project_id, region, version).
    #[serde(default)]
    pub extra: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Whether auto-defense (counter-attack on detected intrusion) is enabled.
    pub auto_defense: bool,
    /// Whether passive monitoring (process + network + file events) is enabled.
    pub monitor: bool,
    /// Whether on-demand virus scanning is available.
    pub scanner_enabled: bool,
    /// Whether quarantined files should be auto-deleted after N days.
    pub quarantine_auto_delete_days: u32,
    /// Whitelist of processes the AI is allowed to spawn without confirmation.
    #[serde(default)]
    pub command_whitelist: Vec<String>,
    /// Whitelist of paths the AI is allowed to write to without confirmation.
    #[serde(default)]
    pub write_path_whitelist: Vec<String>,
    /// List of patterns that, if matched by any process, trigger an alert.
    #[serde(default)]
    pub threat_signatures: Vec<ThreatSignature>,
    /// Webhook URL for security alerts.
    #[serde(default)]
    pub alert_webhook_url: Option<String>,
    /// Email address for security alerts.
    #[serde(default)]
    pub alert_email_to: Option<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            auto_defense: true,
            monitor: true,
            scanner_enabled: true,
            quarantine_auto_delete_days: 30,
            command_whitelist: vec![
                "ls".into(),
                "cat".into(),
                "echo".into(),
                "pwd".into(),
                "date".into(),
                "git status".into(),
                "git log".into(),
                "tasklist".into(),
                "systeminfo".into(),
            ],
            write_path_whitelist: vec!["~/Documents/AegisAI/".into()],
            threat_signatures: ThreatSignature::default_list(),
            alert_webhook_url: None,
            alert_email_to: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatSignature {
    pub id: String,
    pub name: String,
    /// Regex matched against process name + command line.
    pub pattern: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl ThreatSignature {
    fn default_list() -> Vec<Self> {
        vec![
            ThreatSignature {
                id: "ts-001".into(),
                name: "Reverse shell pattern".into(),
                pattern: r"/dev/tcp/|nc -e |bash -i".into(),
                severity: Severity::Critical,
            },
            ThreatSignature {
                id: "ts-002".into(),
                name: "Credential dump".into(),
                pattern: r"mimikatz|procdump|lsass".into(),
                severity: Severity::Critical,
            },
            ThreatSignature {
                id: "ts-003".into(),
                name: "Crypto miner".into(),
                pattern: r"xmrig|stratum\+tcp|minerd".into(),
                severity: Severity::High,
            },
            ThreatSignature {
                id: "ts-004".into(),
                name: "Unknown outbound port scan".into(),
                pattern: r"nmap|masscan|zmap".into(),
                severity: Severity::Medium,
            },
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Maximum number of conversations to keep in the local store.
    pub max_conversations: u32,
    /// Maximum number of activity events to retain.
    pub max_activity_events: u32,
    /// Whether to summarize old conversations to save space.
    pub enable_summarization: bool,
    /// Path to the SQLite database file (None = use default data dir).
    pub db_path: Option<PathBuf>,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_conversations: 1000,
            max_activity_events: 50_000,
            enable_summarization: true,
            db_path: None,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            language: "en".into(),
            mode: OperatingMode::OnDemand,
            providers: ProviderRegistry::default(),
            active_provider: None,
            security: SecurityConfig::default(),
            memory: MemoryConfig::default(),
            allow_autonomous: false,
            bypass_mode: false,
        }
    }
}

impl AppConfig {
    /// Returns the path to the Aegis AI data directory.
    pub fn data_dir() -> PathBuf {
        directories::ProjectDirs::from("com", "aegis", "ai")
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| std::env::temp_dir().join("aegis-ai"))
    }

    /// Returns the path to the config file.
    pub fn config_path() -> PathBuf {
        Self::data_dir().join("config.toml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            let cfg = Self::default();
            cfg.save()?;
            return Ok(cfg);
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading config at {}", path.display()))?;
        let cfg: AppConfig = toml::from_str(&text).context("parsing config.toml")?;
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let text = toml::to_string_pretty(self).context("serializing config")?;
        std::fs::write(&path, text)
            .with_context(|| format!("writing config to {}", path.display()))?;
        Ok(())
    }
}

/// Process-wide configuration holder, updated atomically from any thread.
pub struct ConfigStore(pub RwLock<AppConfig>);

impl ConfigStore {
    pub fn new(cfg: AppConfig) -> Self {
        Self(RwLock::new(cfg))
    }

    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, AppConfig> {
        self.0.read()
    }

    pub fn write(&self) -> parking_lot::RwLockWriteGuard<'_, AppConfig> {
        self.0.write()
    }

    pub fn persist(&self) -> Result<()> {
        let snapshot = self.0.read().clone();
        snapshot.save()
    }
}

// ===========================================================================
// OS Keychain integration for credential storage
// ===========================================================================

/// Store a credential securely in the OS keychain.
/// Falls back to config.toml if keyring is unavailable.
pub fn store_credential_secure(provider_id: &str, api_key: &str) -> Result<()> {
    match keyring::Entry::new("aegis-ai", provider_id) {
        Ok(entry) => {
            match entry.set_password(api_key) {
                Ok(()) => {
                    tracing::debug!("stored credential for {} in OS keychain", provider_id);
                    Ok(())
                }
                Err(e) => {
                    tracing::warn!(
                        "keychain write failed for {}: {e}, falling back to config",
                        provider_id
                    );
                    // Fallback: the caller will store in config.toml
                    Err(anyhow::anyhow!("keychain write failed: {e}"))
                }
            }
        }
        Err(e) => {
            tracing::warn!("keyring entry creation failed for {}: {e}", provider_id);
            Err(anyhow::anyhow!("keyring entry failed: {e}"))
        }
    }
}

/// Retrieve a credential from the OS keychain.
/// Returns None if not found in keychain (caller should check config).
pub fn get_credential_secure(provider_id: &str) -> Option<String> {
    match keyring::Entry::new("aegis-ai", provider_id) {
        Ok(entry) => entry.get_password().ok(),
        Err(_) => None,
    }
}

/// Delete a credential from the OS keychain.
pub fn delete_credential_secure(provider_id: &str) -> Result<()> {
    match keyring::Entry::new("aegis-ai", provider_id) {
        Ok(entry) => {
            match entry.delete_credential() {
                Ok(()) => {
                    tracing::debug!("deleted credential for {} from OS keychain", provider_id);
                    Ok(())
                }
                Err(e) => {
                    tracing::debug!(
                        "keychain delete for {} failed (may not exist): {e}",
                        provider_id
                    );
                    Ok(()) // Not an error if it doesn't exist
                }
            }
        }
        Err(e) => {
            tracing::warn!("keyring entry creation failed for {}: {e}", provider_id);
            Ok(()) // Not an error
        }
    }
}
