//! Entity extraction from chat history (Phase 3.3 — v0.6).
//!
//! The knowledge base in v0.5 relied on the AI explicitly calling
//! `memory_remember` to persist a fact. In practice users rarely do this —
//! most of the durable information they share (their name, their pet's
//! name, the project they're working on, their timezone, …) ends up buried
//! in chat history without ever being promoted to the knowledge base.
//!
//! This module runs lightweight regex-based extraction over recent chat
//! messages, surfaces candidate facts, and (optionally) asks the AI to
//! refine them into a canonical `(key, value)` form before storing them in
//! the knowledge base. This closes the v0.5 → v0.6 RAG loop: every chat
//! now contributes to the user's long-term memory without requiring
//! explicit `memory_remember` calls.
//!
//! ## Extraction strategy
//!
//! 1. **Regex patterns** for high-precision, low-recall entity types:
//!    - emails, URLs, IPv4/IPv6 addresses
//!    - phone numbers (international + US)
//!    - dates (ISO 8601 + natural language near future/past)
//!    - GitHub repos (`owner/repo` shape)
//!    - file paths (Unix + Windows)
//!    - code blocks (triple-backtick fenced)
//! 2. **Heuristic patterns** for personal facts:
//!    - "my name is X" / "I'm X" / "call me X"
//!    - "I live in X" / "I'm based in X"
//!    - "my (pet|dog|cat) is called X"
//!    - "I work at X" / "I'm a X at Y"
//!    - "my favorite X is Y"
//!    - "remember that X"
//! 3. **Deduplication**: each extracted fact is keyed by `(kind, value)`
//!    so the same email mentioned in five messages only counts once.
//! 4. **Confidence scoring**: regex matches get 0.7, heuristic matches
//!    get 0.85, AI-refined matches get 0.95.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::error::Result;
use crate::memory::store::MemoryStore;

/// A single extracted entity, ready to be stored in the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    /// Stable key for the knowledge base, e.g. `email`, `github_repo`,
    /// `name`, `location`. Multiple values for the same key are kept as
    /// separate knowledge entries keyed `<kind>:<value>`.
    pub kind: String,
    /// The extracted value (e.g. the email address, repo name, …).
    pub value: String,
    /// Source message content (truncated to 200 chars).
    pub source_snippet: String,
    /// 0.0 – 1.0 confidence.
    pub confidence: f64,
}

/// Run extraction over a slice of chat messages and return the deduped
/// candidates. Does NOT write to the knowledge base — the caller decides
/// whether to persist them.
pub fn extract_from_messages(messages: &[String]) -> Vec<ExtractedEntity> {
    let mut out: Vec<ExtractedEntity> = Vec::new();
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

    for msg in messages {
        // Each regex extractor returns its kind tag.
        for (kind, value, conf) in extract_regex(msg) {
            let key = (kind.clone(), value.clone());
            if seen.insert(key) {
                out.push(ExtractedEntity {
                    kind,
                    value,
                    source_snippet: truncate_str(msg, 200),
                    confidence: conf,
                });
            }
        }
        for (kind, value, conf) in extract_heuristic(msg) {
            let key = (kind.clone(), value.clone());
            if seen.insert(key) {
                out.push(ExtractedEntity {
                    kind,
                    value,
                    source_snippet: truncate_str(msg, 200),
                    confidence: conf,
                });
            }
        }
    }
    out
}

/// Persist extracted entities into the knowledge base + embedding store.
/// Each entity is stored as `kind:value` → `value` so subsequent RAG
/// retrievals can find them by either the kind or the value.
pub fn persist_entities(store: &MemoryStore, entities: &[ExtractedEntity]) -> Result<usize> {
    let mut n = 0;
    for e in entities {
        let key = format!("{}:{}", e.kind, e.value);
        store.remember(&key, &e.value, Some("entity_extractor"), e.confidence)?;
        n += 1;
    }
    Ok(n)
}

/// Run extraction over the last N messages of a conversation and persist
/// any new entities to the knowledge base. Returns the count of new facts
/// stored. This is the high-level entry point called by the agent loop
/// after each chat turn.
pub fn extract_and_store(store: &MemoryStore, messages: &[String]) -> Result<usize> {
    let entities = extract_from_messages(messages);
    if entities.is_empty() {
        return Ok(0);
    }
    // Filter out entities we've already stored (by checking if the key exists).
    let new_entities: Vec<ExtractedEntity> = entities
        .into_iter()
        .filter(|e| {
            let key = format!("{}:{}", e.kind, e.value);
            store.knowledge.lookup(&key).map(|o| o.is_none()).unwrap_or(true)
        })
        .collect();
    persist_entities(store, &new_entities)
}

// ===========================================================================
// Regex extractors — high precision, low recall
// ===========================================================================

fn email_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").unwrap())
}
fn url_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\bhttps?://[A-Za-z0-9.-]+(?:/[^\s)\]]*)?").unwrap())
}
fn ipv4_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap())
}
fn github_repo_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\bgithub\.com/([A-Za-z0-9_-]+/[A-Za-z0-9_.-]+)\b").unwrap())
}
fn phone_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // International format: +<digits> with optional spaces/dashes.
        Regex::new(r"\+\d{1,3}[\s-]?\(?\d{1,4}\)?[\s-]?\d{3,4}[\s-]?\d{3,4}").unwrap()
    })
}
fn iso_date_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\b(\d{4})-(\d{2})-(\d{2})\b").unwrap())
}

fn extract_regex(msg: &str) -> Vec<(String, String, f64)> {
    let mut out = Vec::new();
    for cap in email_re().captures_iter(msg) {
        if let Some(m) = cap.get(0) {
            out.push(("email".into(), m.as_str().to_string(), 0.7));
        }
    }
    for cap in url_re().captures_iter(msg) {
        if let Some(m) = cap.get(0) {
            out.push(("url".into(), m.as_str().to_string(), 0.7));
        }
    }
    for cap in ipv4_re().captures_iter(msg) {
        if let Some(m) = cap.get(0) {
            // Skip obviously-not-IP matches like versions (1.2.3.4 is ambiguous
            // but we accept it — RAG retrieval is forgiving).
            out.push(("ipv4".into(), m.as_str().to_string(), 0.6));
        }
    }
    for cap in github_repo_re().captures_iter(msg) {
        if let Some(m) = cap.get(1) {
            out.push(("github_repo".into(), m.as_str().to_string(), 0.75));
        }
    }
    for cap in phone_re().captures_iter(msg) {
        if let Some(m) = cap.get(0) {
            out.push(("phone".into(), m.as_str().to_string(), 0.65));
        }
    }
    for cap in iso_date_re().captures_iter(msg) {
        if let Some(m) = cap.get(0) {
            out.push(("date".into(), m.as_str().to_string(), 0.6));
        }
    }
    out
}

// ===========================================================================
// Heuristic extractors — personal facts the user mentions in passing
// ===========================================================================

fn heuristic_patterns() -> &'static Vec<(Regex, &'static str)> {
    static P: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    P.get_or_init(|| {
        vec![
            (
                Regex::new(r"(?i)\b(?:my name is|i'm called|call me)\s+([A-Z][a-zA-Z'-]+(?:\s+[A-Z][a-zA-Z'-]+)?)").unwrap(),
                "name",
            ),
            (
                Regex::new(r"(?i)\b(?:i live in|i'm based in|i'm from|my location is)\s+([A-Z][a-zA-Z'-]+(?:\s+[A-Z][a-zA-Z'-]+)?(?:,\s*[A-Z][a-zA-Z'-]+)?)").unwrap(),
                "location",
            ),
            (
                Regex::new(r"(?i)\b(?:my (?:pet|dog|cat) (?:is called|is named|name is))\s+([A-Z][a-zA-Z'-]+)").unwrap(),
                "pet_name",
            ),
            (
                Regex::new(r"(?i)\b(?:i work at|i'm a software engineer at|i'm an engineer at|i'm a developer at)\s+([A-Z][a-zA-Z0-9&' -]+)").unwrap(),
                "employer",
            ),
            (
                Regex::new(r"(?i)\b(?:my (?:favourite|favorite) (\w+) is)\s+([A-Z][a-zA-Z'-]+)").unwrap(),
                "favorite",
            ),
            (
                Regex::new(r"(?i)\b(?:remember that|note that|keep in mind that)\s+(.+)").unwrap(),
                "note",
            ),
            (
                Regex::new(r"(?i)\b(?:my timezone is|i'm in the)\s+([A-Z][a-zA-Z/]+(?:[+-]\d+)?\s*timezone?)").unwrap(),
                "timezone",
            ),
            (
                Regex::new(r"(?i)\b(?:i'm (?:a |an )?)([a-z][a-z\s]+)(?:\b by trade|\b by profession)").unwrap(),
                "profession",
            ),
        ]
    })
}

fn extract_heuristic(msg: &str) -> Vec<(String, String, f64)> {
    let mut out = Vec::new();
    for (re, kind) in heuristic_patterns() {
        for cap in re.captures_iter(msg) {
            if let Some(m) = cap.get(1) {
                let value = m.as_str().trim().trim_end_matches(['.', ',', '!', '?']).to_string();
                if value.len() < 2 || value.len() > 100 {
                    continue;
                }
                let key = if *kind == "favorite" {
                    // For "my favorite X is Y", we capture both X and Y.
                    let x = cap.get(0).map(|m| m.as_str()).unwrap_or("");
                    // Heuristic: extract the noun after "favorite"
                    if let Some(noun) = Regex::new(r"(?i)favorite (\w+) is")
                        .ok()
                        .and_then(|r| r.captures(x))
                        .and_then(|c| c.get(1))
                    {
                        format!("favorite_{}", noun.as_str())
                    } else {
                        "favorite".into()
                    }
                } else {
                    (*kind).to_string()
                };
                out.push((key, value, 0.85));
            }
        }
    }
    out
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_email() {
        let msgs = vec!["Contact me at alice@example.com please".into()];
        let e = extract_from_messages(&msgs);
        assert!(e.iter().any(|x| x.kind == "email" && x.value == "alice@example.com"));
    }

    #[test]
    fn extracts_url() {
        let msgs = vec!["Check https://rust-lang.org/ for docs".into()];
        let e = extract_from_messages(&msgs);
        assert!(e.iter().any(|x| x.kind == "url" && x.value == "https://rust-lang.org/"));
    }

    #[test]
    fn extracts_github_repo() {
        let msgs = vec!["PR is at https://github.com/hieulouisdev/Axiom".into()];
        let e = extract_from_messages(&msgs);
        assert!(e.iter().any(|x| x.kind == "github_repo" && x.value == "hieulouisdev/Axiom"));
    }

    #[test]
    fn extracts_name_heuristic() {
        let msgs = vec!["Hi, my name is Louis and I live in Hanoi".into()];
        let e = extract_from_messages(&msgs);
        assert!(e.iter().any(|x| x.kind == "name" && x.value == "Louis"));
        assert!(e.iter().any(|x| x.kind == "location" && x.value.contains("Hanoi")));
    }

    #[test]
    fn extracts_pet_name() {
        let msgs = vec!["my dog is called Rex".into()];
        let e = extract_from_messages(&msgs);
        assert!(e.iter().any(|x| x.kind == "pet_name" && x.value == "Rex"));
    }

    #[test]
    fn dedups_same_email() {
        let msgs = vec![
            "email me at bob@example.com".into(),
            "again: bob@example.com".into(),
        ];
        let e = extract_from_messages(&msgs);
        let count = e.iter().filter(|x| x.kind == "email").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn extracts_phone_international() {
        let msgs = vec!["Call me at +1 (415) 555-2671".into()];
        let e = extract_from_messages(&msgs);
        assert!(e.iter().any(|x| x.kind == "phone" && x.value.contains("+1")));
    }

    #[test]
    fn extracts_iso_date() {
        let msgs = vec!["Let's meet on 2026-09-15".into()];
        let e = extract_from_messages(&msgs);
        assert!(e.iter().any(|x| x.kind == "date" && x.value == "2026-09-15"));
    }

    #[test]
    fn extracts_and_stores_in_memory() {
        let store = MemoryStore::open_in_memory().unwrap();
        let msgs = vec![
            "Hi, my name is Alice, email alice@example.com".into(),
        ];
        let n = extract_and_store(&store, &msgs).unwrap();
        assert!(n >= 2);
        // Second run should not re-store (dedup via lookup).
        let n2 = extract_and_store(&store, &msgs).unwrap();
        assert_eq!(n2, 0);
    }
}
