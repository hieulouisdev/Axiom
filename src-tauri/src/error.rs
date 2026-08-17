//! Unified error type for the Aegis AI backend.

use thiserror::Error;

/// Top-level error type surfaced to Tauri commands.
///
/// All internal fallible operations return [`anyhow::Error`] for ergonomic
/// context chaining. The Tauri boundary converts those into [`AegisError`]
/// variants so they can be serialized to the frontend in a structured way.
#[derive(Debug, Error)]
pub enum AegisError {
    #[error("AI provider error: {0}")]
    Ai(String),

    #[error("AI provider not configured: {0}")]
    AiNotConfigured(String),

    #[error("Computer-use operation denied by safety policy: {0}")]
    SafetyDenial(String),

    #[error("Computer-use operation requires user confirmation (token={token})")]
    SafetyConfirmation { token: String, summary: String },

    #[error("Security subsystem error: {0}")]
    Security(String),

    #[error("Memory store error: {0}")]
    Memory(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<anyhow::Error> for AegisError {
    fn from(err: anyhow::Error) -> Self {
        AegisError::Internal(err.to_string())
    }
}

impl From<reqwest::Error> for AegisError {
    fn from(err: reqwest::Error) -> Self {
        AegisError::Network(err.to_string())
    }
}

impl From<serde_json::Error> for AegisError {
    fn from(err: serde_json::Error) -> Self {
        AegisError::Internal(format!("serialization error: {err}"))
    }
}

impl From<std::io::Error> for AegisError {
    fn from(err: std::io::Error) -> Self {
        AegisError::Io(err.to_string())
    }
}

impl From<rusqlite::Error> for AegisError {
    fn from(err: rusqlite::Error) -> Self {
        AegisError::Memory(err.to_string())
    }
}

impl serde::Serialize for AegisError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut kind = match self {
            AegisError::Ai(_) => "ai",
            AegisError::AiNotConfigured(_) => "ai_not_configured",
            AegisError::SafetyDenial(_) => "safety_denial",
            AegisError::SafetyConfirmation { .. } => "safety_confirmation",
            AegisError::Security(_) => "security",
            AegisError::Memory(_) => "memory",
            AegisError::Config(_) => "config",
            AegisError::Io(_) => "io",
            AegisError::Network(_) => "network",
            AegisError::Internal(_) => "internal",
        };

        let mut st = serializer.serialize_struct("AegisError", 3)?;
        st.serialize_field("kind", kind)?;
        st.serialize_field("message", &self.to_string())?;
        if let AegisError::SafetyConfirmation { token, summary } = self {
            st.serialize_field("token", token)?;
            st.serialize_field("summary", summary)?;
        }
        st.end()
    }
}

pub type Result<T> = std::result::Result<T, AegisError>;
