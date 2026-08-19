//! Calendar subsystem (Phase 3.1 — v0.5).
//!
//! Provides:
//! - [`caldav`] — read-only CalDAV client that fetches today's VEVENTs from
//!   any CalDAV-compatible server (Nextcloud, Radicale, Google Calendar via
//!   CalDAV, Synology Calendar, etc.). Uses PROPFIND + REPORT requests via
//!   the existing `reqwest` client — no extra HTTP deps.
//! - [`intent`] — dispatches calendar intents ("schedule a meeting with…",
//!   "what's on my calendar today?") via the AI agent loop or direct
//!   prompt-routing.
//!
//! We intentionally do not implement calendar *writes* (creating events) in
//! v0.5 — that's a security-sensitive operation gated by the safety policy
//! and queued for Phase 4 once we have proper OAuth support. The v0.5 client
//! is read-only and stores cached events in SQLite for offline access.

pub mod caldav;
pub mod intent;

pub use caldav::{CalendarClient, CalendarConfig, CalendarEvent};
pub use intent::{CalendarDispatchResult, CalendarIntent, dispatch_calendar_intent};
