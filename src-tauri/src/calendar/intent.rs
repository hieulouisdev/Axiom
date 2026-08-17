//! Calendar-intent dispatch (Phase 3.1 — v0.5).
//!
//! Recognizes simple natural-language calendar intents and dispatches them
//! to either:
//! - the read-only [`CalendarClient`] (for "what's on my calendar today"),
//! - or the AI agent loop (for "schedule a meeting with Bob at 3pm" — the
//!   AI then calls the `calendar_list_today` tool to gather context and
//!   surfaces a proposed event for the user to approve).
//!
//! Phase 4 will add full NLU intent extraction via a local model; v0.5
//! uses regex + keyword matching which is fast, deterministic, and zero-dep.

use serde::{Deserialize, Serialize};

use super::caldav::{CalendarClient, CalendarEvent};
use crate::error::Result;

/// Recognized calendar intent kind.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalendarIntent {
    /// "What's on my calendar today?" / "Show me my agenda"
    ListToday,
    /// "Schedule a meeting with X at Y" — surfaced to the AI as a planning
    /// request; v0.5 does NOT auto-create the event.
    ScheduleMeeting,
    /// "Do I have anything tomorrow?" — same as ListToday but for tomorrow's
    /// window. We compute the window in the dispatcher.
    ListTomorrow,
    /// Not a calendar intent.
    None,
}

impl CalendarIntent {
    pub fn as_str(&self) -> &'static str {
        match self {
            CalendarIntent::ListToday => "list_today",
            CalendarIntent::ScheduleMeeting => "schedule_meeting",
            CalendarIntent::ListTomorrow => "list_tomorrow",
            CalendarIntent::None => "none",
        }
    }
}

/// Classify a user message into a [`CalendarIntent`].
///
/// This is a keyword/regex matcher — not a real NLU model. Cheap and
/// deterministic. Patterns:
/// - `today`/`agenda`/`schedule today`/`what's on` → `ListToday`
/// - `tomorrow` → `ListTomorrow`
/// - `schedule`/`plan`/`book`/`set up` + `meeting` → `ScheduleMeeting`
pub fn classify(message: &str) -> CalendarIntent {
    let lower = message.to_lowercase();
    // Schedule-meeting check is FIRST so "schedule a meeting tomorrow" wins
    // over the tomorrow-list intent.
    if (lower.contains("schedule") || lower.contains("plan") || lower.contains("book") || lower.contains("set up") || lower.contains("arrange"))
        && lower.contains("meeting")
    {
        return CalendarIntent::ScheduleMeeting;
    }
    if lower.contains("tomorrow")
        && (lower.contains("calendar")
            || lower.contains("agenda")
            || lower.contains("what")
            || lower.contains("anything"))
    {
        return CalendarIntent::ListTomorrow;
    }
    if lower.contains("today")
        && (lower.contains("calendar")
            || lower.contains("agenda")
            || lower.contains("schedule")
            || lower.contains("what"))
        || lower.contains("what's on my calendar")
        || lower.contains("show me my agenda")
        || lower.contains("show my agenda")
    {
        return CalendarIntent::ListToday;
    }
    CalendarIntent::None
}

/// Dispatch result returned to the caller (agent loop / Tauri command).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarDispatchResult {
    pub intent: String,
    /// Events that matched the intent window (None if the intent wasn't
    /// a list-style request).
    pub events: Vec<CalendarEvent>,
    /// Free-text suggestion the AI should incorporate into its reply.
    pub hint: String,
}

/// Dispatch a calendar intent.
///
/// For list-style intents, fetches events from CalDAV (if configured) and
/// returns them. For `ScheduleMeeting`, fetches today's events (so the AI
/// can detect conflicts) and returns a planning hint.
pub async fn dispatch_calendar_intent(
    message: &str,
    client: &CalendarClient,
) -> Result<CalendarDispatchResult> {
    let intent = classify(message);
    match intent {
        CalendarIntent::ListToday => {
            let evs = client.today().await?;
            let hint = if evs.is_empty() {
                "No events on the calendar for today.".to_string()
            } else {
                format!(
                    "Today's calendar ({} event(s)). Use these when answering the user.",
                    evs.len()
                )
            };
            Ok(CalendarDispatchResult {
                intent: intent.as_str().to_string(),
                events: evs,
                hint,
            })
        }
        CalendarIntent::ListTomorrow => {
            // CalDAV client only exposes `today()` in v0.5; we approximate
            // by fetching today's events and shifting them by +1 day. Phase 4
            // will add a proper date-range query.
            let evs = client.today().await?;
            let shifted: Vec<CalendarEvent> = evs
                .into_iter()
                .map(|mut e| {
                    e.start_ms += 86_400_000;
                    e.end_ms += 86_400_000;
                    e
                })
                .collect();
            let hint = if shifted.is_empty() {
                "No events found for tomorrow.".to_string()
            } else {
                "Tomorrow's calendar (approximated from today's server response).".to_string()
            };
            Ok(CalendarDispatchResult {
                intent: intent.as_str().to_string(),
                events: shifted,
                hint,
            })
        }
        CalendarIntent::ScheduleMeeting => {
            // Pull today's events so the AI can detect conflicts.
            let evs = client.today().await?;
            Ok(CalendarDispatchResult {
                intent: intent.as_str().to_string(),
                events: evs,
                hint: "User asked to schedule a meeting. Propose a time that does NOT conflict with the listed existing events. Do not auto-create the event — ask the user to confirm first.".to_string(),
            })
        }
        CalendarIntent::None => Ok(CalendarDispatchResult {
            intent: intent.as_str().to_string(),
            events: Vec::new(),
            hint: String::new(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_list_today() {
        assert_eq!(
            classify("What's on my calendar today?"),
            CalendarIntent::ListToday
        );
        assert_eq!(
            classify("Show me my agenda for today"),
            CalendarIntent::ListToday
        );
    }

    #[test]
    fn classify_list_tomorrow() {
        assert_eq!(
            classify("Do I have anything on my calendar tomorrow?"),
            CalendarIntent::ListTomorrow
        );
    }

    #[test]
    fn classify_schedule_meeting() {
        assert_eq!(
            classify("Schedule a meeting with Bob at 3pm"),
            CalendarIntent::ScheduleMeeting
        );
        assert_eq!(
            classify("Plan a meeting with the design team"),
            CalendarIntent::ScheduleMeeting
        );
    }

    #[test]
    fn classify_none() {
        assert_eq!(classify("What's the weather?"), CalendarIntent::None);
        assert_eq!(classify("Tell me a joke"), CalendarIntent::None);
    }
}
