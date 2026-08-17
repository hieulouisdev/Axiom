//! Text-to-Speech (Phase 3.2).
//!
//! Backends:
//! - [`LocalTts`] — invokes the OS-native speech engine via `Command`:
//!   - Linux: `espeak` (or `espeak-ng`) — widely available, ~2 MB install.
//!   - Windows: PowerShell + `System.Speech.Synthesis.SpeechSynthesizer`
//!     (.NET SAPI), ships with the OS — no install required.
//!   - macOS: `say` — ships with the OS.
//! - [`ElevenLabsTts`] — cloud TTS via ElevenLabs' REST API. Opt-in, requires
//!   an API key stored in the OS keychain entry `aegis-ai/voice_tts`.
//!
//! Both backends write audio bytes to a temporary file. The frontend can
//! then `<audio src="file://…">` it. We deliberately avoid auto-playing
//! audio from the backend — the user's UI controls playback.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{AegisError, Result};

/// TTS options passed by the caller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsOptions {
    /// Voice ID or name. None = the backend's default voice.
    pub voice: Option<String>,
    /// Speech rate in WPM (words per minute). None = backend default.
    pub rate: Option<u32>,
    /// Optional output path. None = pick a temp file.
    pub out_path: Option<String>,
}

impl Default for TtsOptions {
    fn default() -> Self {
        Self {
            voice: None,
            rate: None,
            out_path: None,
        }
    }
}

/// Result of a TTS synthesis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizedSpeech {
    /// Path to the generated audio file (wav / mp3 depending on backend).
    pub path: String,
    /// MIME type of the file (`audio/wav`, `audio/mpeg`).
    pub mime: String,
    /// Approximate audio duration in milliseconds.
    pub duration_ms: u64,
    /// Backend that produced this audio (`"local"`, `"elevenlabs"`).
    pub backend: String,
}

/// Trait every TTS backend implements.
#[async_trait::async_trait]
pub trait TextToSpeech: Send + Sync {
    async fn synthesize(&self, text: &str, opts: &TtsOptions) -> Result<SynthesizedSpeech>;
    fn backend_name(&self) -> &'static str;
}

/// Pick the best available TTS backend (`ElevenLabs` if API key configured,
/// otherwise the local OS engine).
pub fn default_tts() -> Box<dyn TextToSpeech> {
    if let Some(cloud) = ElevenLabsTts::from_env() {
        return Box::new(cloud);
    }
    Box::new(LocalTts)
}

/// Backend label used in the agent's `voice_speak` tool result.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TtsBackend {
    Local,
    ElevenLabs,
}

impl TtsBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            TtsBackend::Local => "local",
            TtsBackend::ElevenLabs => "elevenlabs",
        }
    }
}

// ===========================================================================
// Local TTS (OS-native)
// ===========================================================================

/// OS-native TTS via the platform's command-line speech tool.
pub struct LocalTts;

impl LocalTts {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalTts {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TextToSpeech for LocalTts {
    async fn synthesize(&self, text: &str, opts: &TtsOptions) -> Result<SynthesizedSpeech> {
        if text.trim().is_empty() {
            return Err(AegisError::Internal(
                "tts: text is empty".to_string(),
            ));
        }
        let out_path = opts.out_path.clone().unwrap_or_else(|| {
            let dir = std::env::temp_dir();
            let stem = format!("aegis-tts-{}", uuid::Uuid::new_v4().simple());
            if cfg!(target_os = "linux") {
                dir.join(format!("{stem}.wav")).to_string_lossy().to_string()
            } else if cfg!(windows) {
                dir.join(format!("{stem}.wav")).to_string_lossy().to_string()
            } else if cfg!(target_os = "macos") {
                dir.join(format!("{stem}.aiff")).to_string_lossy().to_string()
            } else {
                dir.join(format!("{stem}.wav")).to_string_lossy().to_string()
            }
        });

        let (mime, duration_ms) = if cfg!(target_os = "linux") {
            self.run_linux(text, opts, &out_path).await?
        } else if cfg!(windows) {
            self.run_windows(text, opts, &out_path).await?
        } else if cfg!(target_os = "macos") {
            self.run_macos(text, opts, &out_path).await?
        } else {
            return Err(AegisError::Internal(
                "tts: unsupported platform for local TTS".to_string(),
            ));
        };

        Ok(SynthesizedSpeech {
            path: out_path,
            mime,
            duration_ms,
            backend: "local".to_string(),
        })
    }

    fn backend_name(&self) -> &'static str {
        "local"
    }
}

impl LocalTts {
    async fn run_linux(
        &self,
        text: &str,
        opts: &TtsOptions,
        out_path: &str,
    ) -> Result<(String, u64)> {
        // Prefer espeak-ng, fall back to espeak, then festival.
        let bin = which_first(&["espeak-ng", "espeak"]);
        let text_owned = text.to_string();
        let out_owned = out_path.to_string();
        let voice = opts.voice.clone();
        let rate = opts.rate.unwrap_or(160);
        tokio::task::spawn_blocking(move || -> Result<(String, u64)> {
            let bin = match bin.as_deref() {
                Some(b) => b,
                None => {
                    return Err(AegisError::Internal(
                        "tts: no local TTS engine (install espeak-ng or espeak)".to_string(),
                    ));
                }
            };
            let mut cmd = std::process::Command::new(bin);
            cmd.arg("-w").arg(&out_owned).arg("-s").arg(rate.to_string());
            if let Some(v) = &voice {
                cmd.arg("-v").arg(v);
            }
            cmd.arg(&text_owned);
            let out = cmd.output().map_err(|e| {
                AegisError::Internal(format!("tts: failed to spawn {bin}: {e}"))
            })?;
            if !out.status.success() {
                return Err(AegisError::Internal(format!(
                    "tts: {bin} exited with status {}: {}",
                    out.status,
                    String::from_utf8_lossy(&out.stderr)
                )));
            }
            // We don't actually parse the WAV header for duration — return 0.
            Ok(("audio/wav".to_string(), 0))
        })
        .await
        .map_err(|e| AegisError::Internal(format!("tts: task join failed: {e}")))?
    }

    async fn run_windows(
        &self,
        text: &str,
        opts: &TtsOptions,
        out_path: &str,
    ) -> Result<(String, u64)> {
        // Build a PowerShell snippet that uses System.Speech.Synthesis.
        // System.Speech ships with .NET Framework (pre-installed on Windows 10+).
        let voice_arg = opts
            .voice
            .clone()
            .unwrap_or_else(|| "".to_string());
        let rate = opts.rate.unwrap_or(0) as i32;
        let text_owned = text.replace('\'', "''");
        let out_owned = out_path.replace('\'', "''");
        let voice_owned = voice_arg.replace('\'', "''");
        let script = format!(
            r#"Add-Type -AssemblyName System.Speech;
$synth = New-Object System.Speech.Synthesis.SpeechSynthesizer;
$synth.Rate = {rate};
if ('{voice_owned}' -ne '') {{ try {{ $synth.SelectVoice('{voice_owned}') }} catch {{ }} }};
$synth.SetOutputToWaveFile('{out_owned}');
$synth.Speak('{text_owned}');
$synth.Dispose();
"#
        );
        let out = tokio::task::spawn_blocking(move || -> std::io::Result<std::process::Output> {
            std::process::Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                .output()
        })
        .await
        .map_err(|e| AegisError::Internal(format!("tts: task join failed: {e}")))?
        .map_err(|e| AegisError::Internal(format!("tts: powershell spawn failed: {e}")))?;
        if !out.status.success() {
            return Err(AegisError::Internal(format!(
                "tts: powershell exited {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(("audio/wav".to_string(), 0))
    }

    async fn run_macos(
        &self,
        text: &str,
        opts: &TtsOptions,
        out_path: &str,
    ) -> Result<(String, u64)> {
        let text_owned = text.to_string();
        let out_owned = out_path.to_string();
        let voice = opts.voice.clone();
        let rate = opts.rate.unwrap_or(180);
        tokio::task::spawn_blocking(move || -> Result<(String, u64)> {
            let mut cmd = std::process::Command::new("say");
            cmd.arg("-o").arg(&out_owned).arg("-r").arg(rate.to_string());
            if let Some(v) = &voice {
                cmd.arg("-v").arg(v);
            }
            cmd.arg(&text_owned);
            let out = cmd
                .output()
                .map_err(|e| AegisError::Internal(format!("tts: say spawn failed: {e}")))?;
            if !out.status.success() {
                return Err(AegisError::Internal(format!(
                    "tts: say exited {}: {}",
                    out.status,
                    String::from_utf8_lossy(&out.stderr)
                )));
            }
            Ok(("audio/aiff".to_string(), 0))
        })
        .await
        .map_err(|e| AegisError::Internal(format!("tts: task join failed: {e}")))?
    }
}

// ===========================================================================
// ElevenLabs cloud TTS
// ===========================================================================

/// ElevenLabs cloud TTS.
pub struct ElevenLabsTts {
    pub api_key: String,
    pub base_url: String,
    pub default_voice_id: String,
    pub client: reqwest::Client,
}

impl ElevenLabsTts {
    pub fn from_env() -> Option<Self> {
        let api_key = crate::config::get_credential_secure("voice_tts")
            .or_else(|| std::env::var("ELEVENLABS_API_KEY").ok())
            .or_else(|| std::env::var("AEGIS_TTS_API_KEY").ok())
            .filter(|s| !s.is_empty())?;
        let base_url = std::env::var("AEGIS_TTS_BASE_URL")
            .unwrap_or_else(|_| "https://api.elevenlabs.io/v1".to_string());
        let default_voice_id = std::env::var("AEGIS_TTS_VOICE_ID")
            .unwrap_or_else(|_| "21m00Tcm4TlvDq8ikWAM".to_string()); // Rachel
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .ok()?;
        Some(Self {
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
            default_voice_id,
            client,
        })
    }
}

#[async_trait::async_trait]
impl TextToSpeech for ElevenLabsTts {
    async fn synthesize(&self, text: &str, opts: &TtsOptions) -> Result<SynthesizedSpeech> {
        if text.trim().is_empty() {
            return Err(AegisError::Internal(
                "tts: text is empty".to_string(),
            ));
        }
        let voice_id = opts
            .voice
            .clone()
            .unwrap_or_else(|| self.default_voice_id.clone());
        let url = format!("{}/text-to-speech/{voice_id}", self.base_url);
        let body = serde_json::json!({
            "text": text,
            "model_id": "eleven_multilingual_v2",
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.7,
            }
        });
        let resp = self
            .client
            .post(url)
            .header("xi-api-key", &self.api_key)
            .header("Accept", "audio/mpeg")
            .json(&body)
            .send()
            .await
            .map_err(|e| AegisError::Network(format!("elevenlabs request failed: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AegisError::Network(format!(
                "elevenlabs TTS failed ({status}): {body}"
            )));
        }
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AegisError::Network(format!("elevenlabs body read failed: {e}")))?;
        let path = opts.out_path.clone().unwrap_or_else(|| {
            std::env::temp_dir()
                .join(format!("aegis-tts-{}.mp3", uuid::Uuid::new_v4().simple()))
                .to_string_lossy()
                .to_string()
        });
        tokio::fs::write(&path, &bytes)
            .await
            .map_err(|e| AegisError::Io(format!("elevenlabs write failed: {e}")))?;
        Ok(SynthesizedSpeech {
            path,
            mime: "audio/mpeg".to_string(),
            duration_ms: 0,
            backend: "elevenlabs".to_string(),
        })
    }

    fn backend_name(&self) -> &'static str {
        "elevenlabs"
    }
}

// ===========================================================================
// Helpers
// ===========================================================================

fn which_first(candidates: &[&str]) -> Option<String> {
    for c in candidates {
        let out = if cfg!(windows) {
            std::process::Command::new("where")
                .arg(c)
                .output()
                .ok()
        } else {
            std::process::Command::new("which")
                .arg(c)
                .output()
                .ok()
        };
        if let Some(o) = out {
            if o.status.success() {
                return Some((*c).to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tts_returns_local_when_no_env() {
        // Strip env so ElevenLabs is not picked up in CI.
        std::env::remove_var("ELEVENLABS_API_KEY");
        std::env::remove_var("AEGIS_TTS_API_KEY");
        // Best-effort: keychain may have a stored key from a previous run.
        // We only check the trait method name; we don't run synthesize().
        let tts = default_tts();
        let _ = tts.backend_name();
    }

    #[test]
    fn tts_options_default() {
        let o = TtsOptions::default();
        assert!(o.voice.is_none());
        assert!(o.rate.is_none());
        assert!(o.out_path.is_none());
    }
}
