//! Minimal read-only CalDAV client (Phase 3.1).
//!
//! Implements just enough of RFC 4791 to fetch today's VEVENTs from a
//! user-configured CalDAV server. Two HTTP requests:
//!
//! 1. `PROPFIND` on the calendar home URL to discover the user's calendar
//!    collections.
//! 2. `REPORT` with a `calendar-query` body to fetch VEVENTs whose `DTSTART`
//!    falls inside `[today_start, today_end]`.
//!
//! VEVENTs are parsed with a tiny line-based parser that handles the common
//! fields (`SUMMARY`, `DTSTART`, `DTEND`, `LOCATION`, `DESCRIPTION`). Line
//! folding (RFC 5545 §3.1) is supported; parameter escaping (`SUMMARY;LANGUAGE=en:…`)
//! is best-effort.
//!
//! Why no `ical` / `rxical` crate? They add ~500 KB to the binary and only
//! v0.5 needs three fields. A future version may swap in a proper iCal
//! parser once we need full RFC 5545 compliance (alarms, recurrence rules).

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{AegisError, Result};

/// Connection configuration for a CalDAV server.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalendarConfig {
    /// Full URL to the user's calendar home (e.g.
    /// `https://cloud.example.com/remote.php/dav/calendars/user@example.com/`).
    pub url: String,
    /// Username (basic-auth).
    pub username: String,
    /// App password (basic-auth). Stored in the OS keychain in production.
    pub password: String,
    /// Calendar display name to filter on (None = use all calendars).
    pub calendar_name: Option<String>,
}

/// A single calendar event extracted from a VEVENT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub location: Option<String>,
    /// Unix timestamp (ms) for the event start.
    pub start_ms: u64,
    /// Unix timestamp (ms) for the event end.
    pub end_ms: u64,
    /// Whether the event is all-day.
    pub all_day: bool,
}

/// CalDAV client. Cheap to clone.
#[derive(Clone)]
pub struct CalendarClient {
    cfg: CalendarConfig,
    http: reqwest::Client,
}

impl CalendarClient {
    pub fn new(cfg: CalendarConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::limited(3))
            .build()
            .map_err(|e| AegisError::Network(format!("calDAV http client: {e}")))?;
        Ok(Self { cfg, http })
    }

    /// Convenience: fetch today's events. Returns an empty vec if the
    /// CalDAV server isn't configured.
    pub async fn today(&self) -> Result<Vec<CalendarEvent>> {
        if self.cfg.url.is_empty() {
            return Ok(Vec::new());
        }
        let cal_urls = self.discover_calendars().await?;
        if cal_urls.is_empty() {
            return Ok(Vec::new());
        }
        let (today_start_ms, today_end_ms) = today_window_utc();
        let mut all: Vec<CalendarEvent> = Vec::new();
        for cal_url in &cal_urls {
            let mut ev = self
                .query_events(cal_url, today_start_ms, today_end_ms)
                .await?;
            all.append(&mut ev);
        }
        all.sort_by_key(|e| e.start_ms);
        Ok(all)
    }

    /// PROPFIND to discover calendar URLs under the configured home.
    ///
    /// Returns relative or absolute URLs as-is from the response.
    async fn discover_calendars(&self) -> Result<Vec<String>> {
        let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:propfind xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:prop>
    <D:displayname/>
    <D:resourcetype/>
  </D:prop>
</D:propfind>"#;
        let resp = self
            .http
            .request(
                reqwest::Method::from_bytes(b"PROPFIND").unwrap(),
                &self.cfg.url,
            )
            .basic_auth(&self.cfg.username, Some(&self.cfg.password))
            .header("Depth", "1")
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(body)
            .send()
            .await
            .map_err(|e| AegisError::Network(format!("CalDAV PROPFIND failed: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AegisError::Network(format!(
                "CalDAV PROPFIND returned {status}: {text}"
            )));
        }
        let text = resp.text().await.unwrap_or_default();
        let urls = parse_hrefs(&text, &self.cfg.url);
        // Filter to resources that look like calendar collections (best-effort).
        // Many CalDAV servers report `<D:resourcetype><C:calendar/></D:resourcetype>`.
        let is_calendar =
            text.contains("<C:calendar") || text.contains("urn:ietf:params:xml:ns:caldav");
        let result: Vec<String> = if is_calendar {
            urls
        } else {
            urls.into_iter().take(8).collect()
        };
        Ok(result)
    }

    /// Issue a `calendar-query` REPORT on a single calendar collection to
    /// fetch VEVENTs in `[start_ms, end_ms]`.
    async fn query_events(
        &self,
        cal_url: &str,
        start_ms: u64,
        end_ms: u64,
    ) -> Result<Vec<CalendarEvent>> {
        let start_iso = iso8601_utc(start_ms);
        let end_iso = iso8601_utc(end_ms);
        let body = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<C:calendar-query xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <D:prop>
    <D:getetag/>
    <C:calendar-data/>
  </D:prop>
  <C:filter>
    <C:comp-filter name="VCALENDAR">
      <C:comp-filter name="VEVENT">
        <C:time-range start="{start_iso}Z" end="{end_iso}Z"/>
      </C:comp-filter>
    </C:comp-filter>
  </C:filter>
</C:calendar-query>"#
        );
        let resp = self
            .http
            .request(reqwest::Method::from_bytes(b"REPORT").unwrap(), cal_url)
            .basic_auth(&self.cfg.username, Some(&self.cfg.password))
            .header("Depth", "1")
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(body)
            .send()
            .await
            .map_err(|e| AegisError::Network(format!("CalDAV REPORT failed: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AegisError::Network(format!(
                "CalDAV REPORT returned {status}: {text}"
            )));
        }
        let text = resp.text().await.unwrap_or_default();
        Ok(parse_vevents(&text))
    }
}

/// Parse `<D:href>…</D:href>` and resolve against `base_url` (best-effort).
fn parse_hrefs(xml: &str, base_url: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut start = 0usize;
    while let Some(s) = xml[start..].find("<D:href>") {
        let abs = start + s + "<D:href>".len();
        if let Some(end_rel) = xml[abs..].find("</D:href>") {
            let href = &xml[abs..abs + end_rel];
            let resolved = resolve_url(base_url, href);
            urls.push(resolved);
            start = abs + end_rel + "</D:href>".len();
        } else {
            break;
        }
    }
    urls
}

/// Resolve a (possibly relative) CalDAV href against `base_url`.
fn resolve_url(base_url: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    // Take the scheme://host[:port] from the base.
    let scheme_end = base_url.find("://").unwrap_or(0);
    let after_scheme = if scheme_end > 0 {
        &base_url[scheme_end + 3..]
    } else {
        base_url
    };
    let host_end = after_scheme.find('/').unwrap_or(after_scheme.len());
    let host = &after_scheme[..host_end];
    let scheme = if scheme_end > 0 {
        &base_url[..scheme_end]
    } else {
        "https"
    };
    if href.starts_with('/') {
        format!("{scheme}://{host}{href}")
    } else {
        format!("{scheme}://{host}/{href}")
    }
}

/// Parse all `<C:calendar-data>…</C:calendar-data>` blocks and extract
/// VEVENTs from each one.
fn parse_vevents(xml: &str) -> Vec<CalendarEvent> {
    let mut events = Vec::new();
    let mut pos = 0usize;
    while let Some(s) = xml[pos..].find("<C:calendar-data") {
        let tag_open = pos + s;
        let tag_close = match xml[tag_open..].find('>') {
            Some(c) => tag_open + c + 1,
            None => break,
        };
        let body_end = match xml[tag_close..].find("</C:calendar-data>") {
            Some(e) => tag_close + e,
            None => break,
        };
        let ical = &xml[tag_close..body_end];
        // Decode XML entities (very common in CalDAV responses).
        let decoded = xml_unescape(ical);
        for ev in extract_vevents(&decoded) {
            events.push(ev);
        }
        pos = body_end + "</C:calendar-data>".len();
    }
    events
}

fn xml_unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#13;\r", "")
        .replace("&#13;", "")
        .replace("&#10;", "\n")
        .replace("&amp;", "&")
}

/// Walk the iCalendar text, unfold continuation lines, and split out VEVENT
/// blocks. Each block is then parsed into a [`CalendarEvent`].
fn extract_vevents(ical: &str) -> Vec<CalendarEvent> {
    let unfolded = unfold_lines(ical);
    let mut events = Vec::new();
    let mut in_event = false;
    let mut buf: Vec<&str> = Vec::new();
    for line in unfolded.lines() {
        if line == "BEGIN:VEVENT" {
            in_event = true;
            buf.clear();
            continue;
        }
        if line == "END:VEVENT" {
            in_event = false;
            if let Some(ev) = parse_vevent(&buf) {
                events.push(ev);
            }
            buf.clear();
            continue;
        }
        if in_event {
            buf.push(line);
        }
    }
    events
}

/// Unfold RFC 5545 continuation lines (lines starting with space or tab are
/// continuations of the previous line).
///
/// Per RFC 5545 §3.1, a folded line is split by inserting CRLF followed by
/// a single linear whitespace character (SPACE or HTAB). Unfolding removes
/// the CRLF but keeps the whitespace character, so a folded `SUMMARY:Hello
/// World` becomes `SUMMARY:Hello World` after unfolding — not
/// `SUMMARY:HelloWorld`. The previous implementation stripped the leading
/// whitespace, which corrupted any value containing a space at the fold
/// point.
fn unfold_lines(ical: &str) -> String {
    let mut out = String::with_capacity(ical.len());
    let mut first = true;
    for line in ical.lines() {
        let is_continuation = line.starts_with(' ') || line.starts_with('\t');
        if is_continuation {
            // Drop the single fold-marker whitespace, keep everything else
            // (including any subsequent spaces in the value).
            let continuation = line.get(1..).unwrap_or("");
            if !out.is_empty() {
                // Preserve a single separator space at the fold point so the
                // unfolded value matches what the producer originally wrote.
                out.push(' ');
                out.push_str(continuation);
            } else {
                // Orphan continuation at the very start of the input — append
                // verbatim (minus the fold marker).
                out.push_str(continuation);
            }
        } else {
            if !first {
                out.push('\n');
            }
            out.push_str(line);
        }
        first = false;
    }
    out
}

fn parse_vevent(lines: &[&str]) -> Option<CalendarEvent> {
    let mut uid = String::new();
    let mut summary = String::new();
    let mut description: Option<String> = None;
    let mut location: Option<String> = None;
    let mut dtstart_raw: Option<String> = None;
    let mut dtend_raw: Option<String> = None;
    let mut all_day = false;

    for line in lines {
        let (name_full, value) = split_property(line);
        // name_full may be `SUMMARY;LANGUAGE=en` or `DTSTART;VALUE=DATE` etc.
        let (name, params) = match name_full.find(';') {
            Some(i) => (&name_full[..i], &name_full[i + 1..]),
            None => (name_full, ""),
        };
        match name.to_uppercase().as_str() {
            "UID" => uid = value.to_string(),
            "SUMMARY" => summary = value.to_string(),
            "DESCRIPTION" => description = Some(value.to_string()),
            "LOCATION" => location = Some(value.to_string()),
            "DTSTART" => {
                dtstart_raw = Some(value.to_string());
                if params.to_uppercase().contains("VALUE=DATE") {
                    all_day = true;
                }
            }
            "DTEND" => dtend_raw = Some(value.to_string()),
            _ => {}
        }
    }
    dtstart_raw.as_ref()?;
    let start_ms = parse_ical_dt(dtstart_raw.as_deref().unwrap_or(""), all_day).unwrap_or(0);
    let end_ms = dtend_raw
        .as_deref()
        .and_then(|s| parse_ical_dt(s, all_day))
        .unwrap_or(start_ms);
    Some(CalendarEvent {
        uid,
        summary,
        description,
        location,
        start_ms,
        end_ms,
        all_day,
    })
}

/// Split `NAME;PARAMS:VALUE` into `(NAME;PARAMS, VALUE)`.
fn split_property(line: &str) -> (&str, &str) {
    // Find the first colon not inside a quoted parameter value.
    let mut in_quote = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_quote = !in_quote,
            ':' if !in_quote => {
                return (&line[..i], &line[i + 1..]);
            }
            _ => {}
        }
    }
    (line, "")
}

/// Parse an iCalendar datetime (`YYYYMMDDTHHMMSSZ` or `YYYYMMDD`) into a
/// UTC Unix timestamp in milliseconds.
fn parse_ical_dt(s: &str, all_day: bool) -> Option<u64> {
    let s = s.trim();
    if s.len() < 8 {
        return None;
    }
    let yyyy = s.get(0..4)?.parse::<u32>().ok()?;
    let mm = s.get(4..6)?.parse::<u32>().ok()?;
    let dd = s.get(6..8)?.parse::<u32>().ok()?;
    let mut hh = 0u32;
    let mut mi = 0u32;
    let mut ss = 0u32;
    if s.len() >= 15 && s.as_bytes().get(8) == Some(&b'T') {
        hh = s.get(9..11)?.parse::<u32>().ok()?;
        mi = s.get(11..13)?.parse::<u32>().ok()?;
        ss = s.get(13..15)?.parse::<u32>().ok()?;
    }
    if all_day {
        hh = 0;
        mi = 0;
        ss = 0;
    }
    let _ = ss;
    let ts = ymd_hms_to_unix(yyyy, mm, dd, hh, mi)?;
    Some(ts * 1000)
}

/// Naive Gregorian→Unix conversion. No leap-second handling.
fn ymd_hms_to_unix(year: u32, month: u32, day: u32, hour: u32, min: u32) -> Option<u64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || min > 59 {
        return None;
    }
    // Days since the Unix epoch (1970-01-01).
    let mut days: i64 = 0;
    for y in 1970..year as i64 {
        days += if is_leap(y) { 366 } else { 365 };
    }
    let mdays = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for (m, &dm) in mdays.iter().enumerate().take(month as usize - 1) {
        days += dm as i64;
        if m == 1 && is_leap(year as i64) {
            days += 1;
        }
    }
    days += (day as i64) - 1;
    let secs = days * 86400 + (hour as i64) * 3600 + (min as i64) * 60;
    Some(secs.max(0) as u64)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

/// Return the UTC start/end of "today" (00:00:00 → 23:59:59) in ms.
fn today_window_utc() -> (u64, u64) {
    let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() as u64 * 1000;
    let day_start_ms = (now_ms / 86_400_000) * 86_400_000;
    (day_start_ms, day_start_ms + 86_400_000 - 1)
}

fn iso8601_utc(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    // Convert Unix seconds back to UTC year/month/day/hour/min manually so
    // we don't depend on the `well_known` feature of the `time` crate.
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let hour = (rem / 3600) as u32;
    let min = ((rem % 3600) / 60) as u32;
    let sec = (rem % 60) as u32;
    let (y, m, d) = days_to_ymd(days);
    format!("{y:04}-{m:02}-{d:02}T{hour:02}:{min:02}:{sec:02}")
}

/// Convert days-since-1970-01-01 back to (year, month, day).
/// Inverse of `ymd_hms_to_unix`'s day-counting logic.
fn days_to_ymd(days: i64) -> (u32, u32, u32) {
    let mut year = 1970i64;
    let mut remaining = days;
    while remaining >= if is_leap(year) { 366 } else { 365 } {
        remaining -= if is_leap(year) { 366 } else { 365 };
        year += 1;
    }
    // `remaining` is now day-of-year (0-indexed).
    let mdays = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u32;
    let mut day_of_year = remaining as u32;
    for (i, &days_in_month) in mdays.iter().enumerate() {
        let mut dim = days_in_month;
        if i == 1 && is_leap(year) {
            dim += 1;
        }
        if day_of_year < dim {
            month = (i + 1) as u32;
            break;
        }
        day_of_year -= dim;
    }
    let day = day_of_year + 1;
    (year as u32, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unfold_continuation_lines() {
        let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:Hello\r\n World\r\nEND:VEVENT\r\n";
        let out = unfold_lines(ical);
        assert!(out.contains("SUMMARY:Hello World"));
    }

    #[test]
    fn parse_simple_vevent() {
        let ical = "BEGIN:VEVENT\r\nUID:abc-123\r\nSUMMARY:Lunch\r\nDTSTART:20260817T120000Z\r\nDTEND:20260817T130000Z\r\nEND:VEVENT\r\n";
        let evs = extract_vevents(ical);
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].summary, "Lunch");
        assert_eq!(evs[0].uid, "abc-123");
        // 2026-08-17 12:00 UTC = some timestamp > 0
        assert!(evs[0].start_ms > 0);
    }

    #[test]
    fn parse_all_day_event() {
        let ical = "BEGIN:VEVENT\r\nUID:all-1\r\nSUMMARY:Holiday\r\nDTSTART;VALUE=DATE:20260101\r\nDTEND;VALUE=DATE:20260102\r\nEND:VEVENT\r\n";
        let evs = extract_vevents(ical);
        assert_eq!(evs.len(), 1);
        assert!(evs[0].all_day);
    }

    #[test]
    fn resolve_relative_url() {
        let out = resolve_url(
            "https://cloud.example.com/dav/cal/me/",
            "/dav/cal/me/personal/",
        );
        assert_eq!(out, "https://cloud.example.com/dav/cal/me/personal/");
        let out2 = resolve_url("https://cloud.example.com/dav/cal/me/", "personal/");
        assert_eq!(out2, "https://cloud.example.com/personal/");
        let out3 = resolve_url(
            "https://cloud.example.com/dav/cal/me/",
            "https://other.example.com/cal/x",
        );
        assert_eq!(out3, "https://other.example.com/cal/x");
    }

    #[test]
    fn leap_year_check() {
        assert!(is_leap(2024));
        assert!(!is_leap(2100));
        assert!(is_leap(2000));
    }
}
