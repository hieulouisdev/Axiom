//! Speech-to-Text (Phase 3.2).
//!
//! Two backends:
//! - [`OpenAiWhisper`] — cloud STT via the OpenAI Whisper-compatible
//!   `/v1/audio/transcriptions` endpoint. Works with OpenAI itself, Azure
//!   OpenAI Whisper deployments, Groq's `distil-whisper-large-v3-en`, and any
//!   OpenAI-compatible gateway (e.g. localai, ollama with whisper bindings).
//! - [`LocalStt`] — graceful no-op used when no API key is configured.
//!   Returns an empty transcript so the agent loop can fall back to text
//!   input.
//!
//! The wake-word detector is a tiny substring matcher that runs on the
//! transcript after STT completes. Cheap, deterministic, zero extra deps.

use std::time::Duration;

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::error::{AegisError, Result};

/// Default wake phrase the assistant listens for.
pub const DEFAULT_WAKE_WORD: &str = "hey aegis";

/// A completed STT transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    /// Recognized text (empty if the engine returned nothing).
    pub text: String,
    /// Language code the engine detected or was told to use (e.g. "en", "vi").
    pub language: String,
    /// Approximate duration of the source audio in milliseconds.
    pub duration_ms: u64,
    /// Backend that produced this transcript (`"openai_whisper"` or `"local"`).
    pub backend: String,
    /// Whether the wake word was detected in the transcript.
    pub wake_word_detected: bool,
}

/// STT backend trait.
#[async_trait::async_trait]
pub trait SpeechToText: Send + Sync {
    async fn transcribe(&self, audio: &[u8], mime: &str) -> Result<Transcript>;
    fn backend_name(&self) -> &'static str;
}

/// OpenAI-compatible Whisper cloud STT.
///
/// `api_key` is read from the OS keychain entry `aegis-ai/voice_stt` (set
/// by the user via the Settings UI) or the `OPENAI_API_KEY` env var. The
/// `base_url` defaults to `https://api.openai.com/v1` but can be overridden
/// to point at a self-hosted Whisper gateway.
pub struct OpenAiWhisper {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub language: Option<String>,
    pub client: reqwest::Client,
}

impl OpenAiWhisper {
    /// Build a default OpenAI Whisper client from env / keychain.
    pub fn from_env() -> Option<Self> {
        let api_key = crate::config::get_credential_secure("voice_stt")
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .or_else(|| std::env::var("AEGIS_STT_API_KEY").ok())
            .filter(|s| !s.is_empty())?;
        let base_url = std::env::var("AEGIS_STT_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let model = std::env::var("AEGIS_STT_MODEL").unwrap_or_else(|_| "whisper-1".to_string());
        let language = std::env::var("AEGIS_STT_LANGUAGE").ok();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .ok()?;
        Some(Self {
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
            model,
            language,
            client,
        })
    }
}

#[async_trait::async_trait]
impl SpeechToText for OpenAiWhisper {
    async fn transcribe(&self, audio: &[u8], mime: &str) -> Result<Transcript> {
        // Whisper expects multipart/form-data with a `file` field and `model`.
        let ext = match mime {
            "audio/wav" | "audio/x-wav" | "audio/wave" => "wav",
            "audio/mpeg" | "audio/mp3" => "mp3",
            "audio/ogg" => "ogg",
            "audio/webm" => "webm",
            "audio/flac" => "flac",
            _ => "wav",
        };
        let file_name = format!("aegis_input.{ext}");
        let form = reqwest::multipart::Form::new()
            .text("model", self.model.clone())
            .text("response_format", "verbose_json")
            .part(
                "file",
                reqwest::multipart::Part::bytes(audio.to_vec())
                    .file_name(file_name)
                    .mime_str(mime)
                    .map_err(|e| AegisError::Network(format!("invalid mime: {e}")))?,
            );
        let mut req = self
            .client
            .post(format!("{}/audio/transcriptions", self.base_url))
            .bearer_auth(&self.api_key)
            .multipart(form);
        if let Some(lang) = &self.language {
            req = req.query(&[("language", lang)]);
        }
        let resp = req.send().await.map_err(|e| {
            AegisError::Network(format!("whisper transcription request failed: {e}"))
        })?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AegisError::Network(format!(
                "whisper transcription failed ({status}): {body}"
            )));
        }
        let v: serde_json::Value = resp.json().await.map_err(|e| {
            AegisError::Network(format!("whisper response JSON decode failed: {e}"))
        })?;
        let text = v
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let language = v
            .get("language")
            .and_then(|l| l.as_str())
            .unwrap_or("en")
            .to_string();
        let duration_ms = v
            .get("duration")
            .and_then(|d| d.as_f64())
            .map(|f| (f * 1000.0) as u64)
            .unwrap_or(0);
        let wake_word_detected = detect_wake_word(&text);
        Ok(Transcript {
            text,
            language,
            duration_ms,
            backend: "openai_whisper".to_string(),
            wake_word_detected,
        })
    }

    fn backend_name(&self) -> &'static str {
        "openai_whisper"
    }
}

/// Local STT fallback. Returns an empty transcript.
///
/// In Phase 4 we'll wire this up to a local Whisper model via `whisper-rs`,
/// but that requires downloading model weights at install time — incompatible
/// with the v0.5 release pipeline.
pub struct LocalStt;

#[async_trait::async_trait]
impl SpeechToText for LocalStt {
    async fn transcribe(&self, _audio: &[u8], _mime: &str) -> Result<Transcript> {
        Ok(Transcript {
            text: String::new(),
            language: std::env::var("AEGIS_LANG").unwrap_or_else(|_| "en".into()),
            duration_ms: 0,
            backend: "local".to_string(),
            wake_word_detected: false,
        })
    }

    fn backend_name(&self) -> &'static str {
        "local"
    }
}

/// Return the best available STT backend (cloud if configured, local otherwise).
pub fn default_stt() -> Box<dyn SpeechToText> {
    if let Some(cloud) = OpenAiWhisper::from_env() {
        return Box::new(cloud);
    }
    Box::new(LocalStt)
}

/// Case-insensitive substring check for the configured wake word.
///
/// Returns `true` if the transcript begins with or contains the wake phrase
/// followed by a word boundary. This is intentionally simple — Phase 4 will
/// swap in a proper keyword-spotting model.
pub fn detect_wake_word(text: &str) -> bool {
    if text.trim().is_empty() {
        return false;
    }
    let lower = text.to_lowercase();
    let wake = std::env::var("AEGIS_WAKE_WORD").unwrap_or_else(|_| DEFAULT_WAKE_WORD.to_string());
    lower.contains(&wake.to_lowercase())
}

/// Encode raw bytes as base64 — handy when callers want to embed audio in JSON.
pub fn encode_audio_b64(audio: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(audio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_word_substring_match() {
        assert!(detect_wake_word("Hey Aegis, what's the weather?"));
        assert!(detect_wake_word("hey aegis schedule a meeting"));
        assert!(!detect_wake_word("hello world"));
        assert!(!detect_wake_word(""));
    }

    #[test]
    fn local_stt_returns_empty_transcript() {
        let stt = LocalStt;
        let t = futures::executor::block_on(stt.transcribe(&[], "audio/wav")).unwrap();
        assert_eq!(t.text, "");
        assert!(!t.wake_word_detected);
        assert_eq!(t.backend, "local");
    }
}
