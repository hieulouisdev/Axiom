//! v1.6.0 — Proactive Intelligence Layer.
//!
//! Aegis AI v1.6 ships a proactive intelligence layer that watches the
//! memory store + activity log and surfaces insights the user might not
//! have asked for: "you've been working on Rust projects a lot this week",
//! "you have 3 unresolved security warnings", "you mentioned Alice 12 times
//! — want to add her to your contacts?". Insights are stored in-memory and
//! pushed to the frontend via the `intelligence://insight` Tauri event.
//!
//! The engine is intentionally conservative about user privacy — it never
//! leaves the local process, never logs raw conversation content, and only
//! emits aggregate signals (counts, durations, frequencies).

pub mod proactive;

pub use proactive::{Insight, InsightKind, ProactiveEngine};
