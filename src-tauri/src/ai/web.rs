//! Web access module: real web search + content extraction.
//!
//! v0.6 replaces the v0.4 `web_search` stub with a working implementation
//! backed by the DuckDuckGo HTML endpoint (no API key required). It also
//! exposes a `readability` helper that strips HTML boilerplate and returns
//! the readable text of a page so the AI can ingest web pages without
//! pulling in a heavy browser-engine dependency.
//!
//! Design notes:
//! - DuckDuckGo's HTML endpoint is rate-limited but tolerant of plain HTTP
//!   clients; we send a desktop User-Agent so the response is the full
//!   results page rather than the lite mobile version.
//! - We extract result anchors via a forgiving regex rather than a full
//!   HTML parser — the DDG HTML is well-formed enough that this works
//!   reliably, and avoiding `scraper`/`html5ever` keeps compile times down.
//! - For page readability we walk the HTML with a tiny state machine that
//!   drops `<script>`, `<style>`, `<nav>`, `<header>`, `<footer>` blocks
//!   and decodes common HTML entities. This is *not* a full readability
//!   implementation (no scoring), but it's enough for the AI to consume
//!   most article-style pages.
//!
//! All network access goes through `reqwest` with a 15-second timeout so a
//! slow upstream cannot stall the agent loop.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::error::{AegisError, Result};

/// A single search hit returned by [`web_search`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    /// Short snippet (max ~300 chars) describing the page.
    pub snippet: String,
}

/// Maximum number of search results to return per query.
pub const MAX_RESULTS: usize = 8;

/// Run a web search and return up to [`MAX_RESULTS`] hits.
///
/// Uses DuckDuckGo's HTML endpoint. No API key is required.
pub async fn web_search(query: &str) -> Result<Vec<SearchResult>> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/124.0.0.0 Safari/537.36",
        )
        .build()
        .map_err(|e| AegisError::Internal(format!("http client build failed: {e}")))?;

    let resp = client
        .get("https://html.duckduckgo.com/html/")
        .query(&[("q", query)])
        .send()
        .await
        .map_err(|e| AegisError::Ai(format!("web search request failed: {e}")))?;

    let html = resp
        .text()
        .await
        .map_err(|e| AegisError::Ai(format!("web search body read failed: {e}")))?;

    Ok(extract_ddg_results(&html))
}

/// Run a web search synchronously — used by the agent tool dispatcher
/// (which runs in a sync context). Spawns a one-shot runtime if no
/// runtime is currently active.
pub fn web_search_sync(query: &str) -> Vec<SearchResult> {
    match tokio::runtime::Handle::try_current() {
        Ok(h) => h.block_on(web_search(query)).unwrap_or_default(),
        Err(_) => {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("failed to spawn runtime for web_search: {e}");
                    return Vec::new();
                }
            };
            rt.block_on(web_search(query)).unwrap_or_default()
        }
    }
}

/// Parse DuckDuckGo's HTML results page into structured hits.
///
/// The DDG HTML endpoint wraps each result in `<a class="result__a" href="…">title</a>`
/// and places the snippet in `<a class="result__snippet" …>snippet</a>`. The hrefs
/// go through DDG's `//duckduckgo.com/l/?uddg=<encoded>` redirect, which we
/// URL-decode so the AI sees the real URL.
fn extract_ddg_results(html: &str) -> Vec<SearchResult> {
    let mut out: Vec<SearchResult> = Vec::new();
    // Anchor titles: <a class="result__a" ...>title text</a>
    let title_re =
        regex::Regex::new(r#"<a[^>]*class="result__a"[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#)
            .expect("title regex is valid");

    let snippet_re = regex::Regex::new(r#"<a[^>]*class="result__snippet"[^>]*>(.*?)</a>"#)
        .expect("snippet regex is valid");

    // Walk the HTML in chunks split on "result__url" so we get one entry per result.
    for block in html.split("result__body") {
        if out.len() >= MAX_RESULTS {
            break;
        }
        // Skip blocks that don't have a title link.
        let Some(title_caps) = title_re.captures(block) else {
            continue;
        };
        let raw_href = title_caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let title_html = title_caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let title = strip_tags(title_html).trim().to_string();
        if title.is_empty() {
            continue;
        }
        let url = decode_ddg_redirect(raw_href);
        if url.is_empty() || !url.starts_with("http") {
            continue;
        }
        let snippet = snippet_re
            .captures(block)
            .and_then(|c| c.get(1).map(|m| m.as_str()))
            .map(strip_tags)
            .unwrap_or_default()
            .trim()
            .to_string();
        out.push(SearchResult {
            title,
            url,
            snippet: truncate_str(&snippet, 300),
        });
    }
    out
}

/// Resolve a DuckDuckGo redirect URL to the real underlying URL.
/// DDG wraps external links as `//duckduckgo.com/l/?uddg=<encoded>&rut=...`
/// or `https://duckduckgo.com/l/?uddg=...`. If we cannot find the `uddg`
/// parameter we fall back to the raw href.
fn decode_ddg_redirect(href: &str) -> String {
    // Normalise protocol-relative URLs.
    let href = if let Some(stripped) = href.strip_prefix("//") {
        format!("https://{stripped}")
    } else {
        href.to_string()
    };
    if let Some(idx) = href.find("uddg=") {
        let tail = &href[idx + 5..];
        let end = tail.find('&').unwrap_or(tail.len());
        let encoded = &tail[..end];
        if let Ok(decoded) = urlencoding::decode(encoded) {
            return decoded.into_owned();
        }
    }
    href
}

/// Strip HTML tags and decode common entities. Collapses whitespace.
pub fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    let decoded = decode_entities(&out);
    decoded.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Decode the handful of HTML entities that show up in DDG snippets.
fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&ndash;", "–")
        .replace("&mdash;", "—")
        .replace("&hellip;", "…")
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}

/// Fetch a URL, extract the readable text, and return it (capped at 32 KB).
///
/// This is the read-side companion to [`web_search`]: once the AI has a
/// list of candidate URLs, it can call `http_fetch` (which now uses this
/// helper internally) to ingest the page content.
pub async fn fetch_readable(url: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) \
             Chrome/124.0.0.0 Safari/537.36",
        )
        .build()
        .map_err(|e| AegisError::Internal(format!("http client build failed: {e}")))?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| AegisError::Ai(format!("http fetch failed: {e}")))?;
    let html = resp
        .text()
        .await
        .map_err(|e| AegisError::Ai(format!("http fetch body read failed: {e}")))?;
    Ok(extract_readable(&html))
}

/// Synchronous wrapper for `fetch_readable` — used by the agent tool dispatcher.
pub fn fetch_readable_sync(url: &str) -> String {
    match tokio::runtime::Handle::try_current() {
        Ok(h) => h
            .block_on(fetch_readable(url))
            .unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}")),
        Err(_) => {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(r) => r,
                Err(e) => return format!("{{\"error\":\"runtime error: {e}\"}}"),
            };
            rt.block_on(fetch_readable(url))
                .unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
        }
    }
}

/// Extract readable text from an HTML page by stripping non-content blocks
/// (script, style, nav, header, footer, aside, form) and removing tags.
/// Returns a single string with `\n` between block-level elements.
pub fn extract_readable(html: &str) -> String {
    let lower = html.to_lowercase();
    let skip_openings: &[&str] = &[
        "<script",
        "<style",
        "<nav",
        "<header",
        "<footer",
        "<aside",
        "<form",
        "<noscript",
        "<svg",
        "<iframe",
        "<button",
    ];
    let skip_closings: &[&str] = &[
        "</script>",
        "</style>",
        "</nav>",
        "</header>",
        "</footer>",
        "</aside>",
        "</form>",
        "</noscript>",
        "</svg>",
        "</iframe>",
        "</button>",
    ];

    let mut buf = String::with_capacity(html.len());
    let mut i = 0;
    let bytes = html.as_bytes();
    let low_bytes = lower.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Check if we're at the start of a skip tag.
            let mut skipped = false;
            for (idx, open) in skip_openings.iter().enumerate() {
                if low_bytes[i..].starts_with(open.as_bytes()) {
                    let close = skip_closings[idx];
                    if let Some(pos) = lower[i..].find(close) {
                        i += pos + close.len();
                    } else {
                        // No close tag — drop to end.
                        i = bytes.len();
                    }
                    skipped = true;
                    break;
                }
            }
            if skipped {
                continue;
            }
        }
        // Copy this byte as a char (HTML is ASCII for tag structure).
        buf.push(bytes[i] as char);
        i += 1;
    }

    // Now strip remaining tags and convert block-level closers to newlines.
    let mut text = String::with_capacity(buf.len());
    let mut in_tag = false;
    for ch in buf.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                if in_tag {
                    in_tag = false;
                }
            }
            c if !in_tag => text.push(c),
            _ => {}
        }
    }
    // Replace block close markers with newlines.
    let text = text
        .replace("</p>", "\n\n")
        .replace("</div>", "\n")
        .replace("</li>", "\n")
        .replace("</h1>", "\n\n")
        .replace("</h2>", "\n\n")
        .replace("</h3>", "\n\n")
        .replace("</h4>", "\n\n")
        .replace("</h5>", "\n\n")
        .replace("</h6>", "\n\n")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n");
    let decoded = decode_entities(&text);
    // Collapse runs of whitespace, preserve paragraph breaks.
    let mut out = String::with_capacity(decoded.len());
    let mut blank = false;
    for line in decoded.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !blank {
                out.push('\n');
                blank = true;
            }
        } else {
            out.push_str(trimmed);
            out.push(' ');
            blank = false;
        }
    }
    // Cap at 32 KB.
    let out = out.trim().to_string();
    if out.chars().count() <= 32 * 1024 {
        out
    } else {
        let mut capped: String = out.chars().take(32 * 1024).collect();
        capped.push_str("\n\n…[truncated]");
        capped
    }
}

// Lightweight URL-decoding helper to avoid pulling in another crate.
mod urlencoding {
    pub fn decode(s: &str) -> Result<std::borrow::Cow<'_, str>, std::str::Utf8Error> {
        if !s.contains('%') && !s.contains('+') {
            return Ok(std::borrow::Cow::Borrowed(s));
        }
        let mut out = String::with_capacity(s.len());
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'+' => {
                    out.push(' ');
                    i += 1;
                }
                b'%' if i + 2 < bytes.len() => {
                    let hi = hex_digit(bytes[i + 1]);
                    let lo = hex_digit(bytes[i + 2]);
                    if let (Some(h), Some(l)) = (hi, lo) {
                        out.push(((h << 4) | l) as char);
                        i += 3;
                    } else {
                        out.push('%');
                        i += 1;
                    }
                }
                c => {
                    out.push(c as char);
                    i += 1;
                }
            }
        }
        Ok(std::borrow::Cow::Owned(out))
    }
    fn hex_digit(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_simple_tags() {
        let out = strip_tags("<b>Hello</b> <i>world</i>");
        assert_eq!(out, "Hello world");
    }

    #[test]
    fn decodes_entities() {
        let out = strip_tags("a &amp; b &lt; c &gt; d");
        assert_eq!(out, "a & b < c > d");
    }

    #[test]
    fn extract_readable_drops_script_blocks() {
        let html = r#"<html><body><script>alert('hi')</script><p>Hello world</p></body></html>"#;
        let out = extract_readable(html);
        assert!(out.contains("Hello world"));
        assert!(!out.contains("alert"));
    }

    #[test]
    fn extract_readable_truncates_long_text() {
        let html = format!("<p>{}</p>", "a".repeat(64 * 1024));
        let out = extract_readable(&html);
        assert!(out.ends_with("…[truncated]"));
    }

    #[test]
    fn ddg_redirect_decoded() {
        let url = decode_ddg_redirect(
            "//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fpath&rut=abc",
        );
        assert_eq!(url, "https://example.com/path");
    }

    #[test]
    fn ddg_results_parsed() {
        let html = r#"
        <div class="result__body">
          <a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Ffoo">Example Foo</a>
          <a class="result__snippet">This is the <b>snippet</b> text.</a>
        </div>
        <div class="result__body">
          <a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2Fbar">Example Bar</a>
          <a class="result__snippet">Second snippet.</a>
        </div>
        "#;
        let results = extract_ddg_results(html);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].title, "Example Foo");
        assert_eq!(results[0].url, "https://example.com/foo");
        assert!(results[0].snippet.contains("snippet"));
    }
}
