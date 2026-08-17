//! Voice I/O subsystem (Phase 3.2 — v0.5).
//!
//! Provides:
//! - [`stt`] — Speech-to-Text via cloud (OpenAI Whisper-compatible) or local
//!   fallback (returns empty when no provider is configured).
//! - [`tts`] — Text-to-Speech via OS-native engines (`espeak` on Linux,
//!   `SAPI.SpVoice` via PowerShell on Windows, `say` on macOS) with an
//!   optional ElevenLabs cloud backend.
//! - [`hotkey`] — System-wide push-to-talk hotkey (default: `Ctrl+Space`)
//!   registered through `tauri-plugin-global-shortcut`.
//!
//! Design goals:
//! 1. **No mandatory heavy ML deps.** Local STT/TTS use whatever the OS ships
//!    with; cloud STT/TTS use plain `reqwest` JSON/HTTP. This keeps the build
//!    reproducible on the v0.5 release pipeline.
//! 2. **Belt-and-suspenders fallback.** Every cloud call degrades to a
//!    well-defined local stub when the API key is missing or the network
//!    fails. The agent loop never blocks on voice I/O.
//! 3. **Privacy by default.** Local engines are preferred. Cloud engines
//!    are opt-in via explicit API keys stored in the OS keychain.

pub mod hotkey;
pub mod stt;
pub mod tts;

pub use hotkey::{HotkeyManager, PushToTalkState};
pub use stt::{SpeechToText, Transcript};
pub use tts::{TextToSpeech, TtsBackend, TtsOptions};
