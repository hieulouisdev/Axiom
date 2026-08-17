//! YARA rule loader (Phase 2.3 — v0.6).
//!
//! YARA is a pattern-matching tool used by malware researchers to identify
//! and classify file samples. v0.6 lays the groundwork for loading custom
//! YARA rules from the user's data directory and running them against
//! files during on-demand scans.
//!
//! Full YARA integration requires the `yara-rs` crate (which in turn
//! depends on the libyara C library). To keep the v0.6 build
//! self-contained, we ship a pure-Rust loader that:
//!
//! 1. Discovers `.yar` and `.yara` files under `~/.local/share/aegis-ai/yara/`.
//! 2. Parses the rule headers (name + tags) using a forgiving regex.
//! 3. Surfaces the parsed rule list to the UI so the user can see what
//!    rules are loaded.
//! 4. Performs substring matching against the rule's literal string
//!    patterns as a stop-gap until full YARA semantics land in Phase 4.
//!
//! The stop-gap matcher is intentionally conservative: it only matches
//! plain-text strings inside double quotes that don't contain regex or
//! hex patterns. This catches the majority of "indicator-of-compromise"
//! rules in the wild without needing the full YARA engine.

use std::fs;
use std::path::PathBuf;

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::config::AppConfig;
use crate::error::{AegisError, Result};

/// A single parsed YARA rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YaraRule {
    pub name: String,
    /// Tags declared after the rule name (`rule foo : bar baz {`).
    pub tags: Vec<String>,
    /// Literal strings extracted from the rule body (for stop-gap matching).
    pub strings: Vec<String>,
    /// Source file path.
    pub source: PathBuf,
}

/// Directory where the user drops custom `.yar` / `.yara` files.
pub fn yara_dir() -> PathBuf {
    AppConfig::data_dir().join("yara")
}

/// Discover and parse all `.yar` / `.yara` files in the YARA directory.
/// Returns an empty vec if the directory doesn't exist or is empty.
pub fn load_all() -> Result<Vec<YaraRule>> {
    let dir = yara_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "yar" && ext != "yara" {
            continue;
        }
        let text = fs::read_to_string(&path)?;
        let rules = parse_rules(&text, &path);
        out.extend(rules);
    }
    Ok(out)
}

/// Parse YARA rules from a single source file. Forgiving: malformed rules
/// are silently skipped.
pub fn parse_rules(text: &str, source: &PathBuf) -> Vec<YaraRule> {
    let header_re = yara_header_re();
    let string_re = yara_string_re();

    let mut out = Vec::new();
    for cap in header_re.captures_iter(text) {
        let name = cap.name("name").map(|m| m.as_str().to_string()).unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let tags_str = cap.name("tags").map(|m| m.as_str()).unwrap_or("");
        let tags: Vec<String> = tags_str
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        // Find the rule body — from the end of this header match to the
        // next `rule ` line or end of file.
        let body_start = cap.get(0).map(|m| m.end()).unwrap_or(0);
        let body_end = text[body_start..]
            .find("\nrule ")
            .map(|p| body_start + p)
            .unwrap_or(text.len());
        let body = &text[body_start..body_end];
        let mut strings = Vec::new();
        for scap in string_re.captures_iter(body) {
            if let Some(m) = scap.name("str") {
                strings.push(m.as_str().to_string());
            }
        }
        out.push(YaraRule {
            name,
            tags,
            strings,
            source: source.clone(),
        });
    }
    out
}

/// Stop-gap matcher: scan a file's bytes for any of the literal strings
/// in the loaded YARA rules. Returns the names of the rules that matched.
pub fn scan_file(rules: &[YaraRule], content: &[u8]) -> Vec<String> {
    let mut hits = Vec::new();
    let text = String::from_utf8_lossy(content);
    for rule in rules {
        for s in &rule.strings {
            if !s.is_empty() && text.contains(s.as_str()) {
                hits.push(rule.name.clone());
                break;
            }
        }
    }
    hits
}

fn yara_header_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?m)^rule\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)(?:\s*:\s*(?P<tags>[^\n{]+))?\s*\{")
            .unwrap()
    })
}

fn yara_string_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // Match `string_name = "literal value"` — only plain double-quoted
        // strings, not regex / hex / byte patterns.
        Regex::new(r#"\$\w+\s*=\s*"(?P<str>[^"\\]+)""#).unwrap()
    })
}

/// Ensure the YARA directory exists. Called at boot so the user can drop
/// rule files into it without manually creating the directory.
pub fn ensure_dir() -> Result<()> {
    let dir = yara_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .map_err(|e| AegisError::Io(format!("failed to create {}: {e}", dir.display())))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_rule() {
        let src = r#"
rule example_rule : tag1 tag2 {
    strings:
        $a = "suspicious string"
        $b = "another indicator"
    condition:
        any of them
}
"#;
        let rules = parse_rules(src, &PathBuf::from("test.yar"));
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "example_rule");
        assert_eq!(rules[0].tags, vec!["tag1", "tag2"]);
        assert_eq!(rules[0].strings.len(), 2);
        assert!(rules[0].strings.contains(&"suspicious string".to_string()));
    }

    #[test]
    fn parses_multiple_rules() {
        let src = r#"
rule first_rule {
    strings:
        $a = "foo"
    condition:
        $a
}

rule second_rule : malware {
    strings:
        $b = "bar"
    condition:
        $b
}
"#;
        let rules = parse_rules(src, &PathBuf::from("multi.yar"));
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].name, "first_rule");
        assert_eq!(rules[1].name, "second_rule");
        assert_eq!(rules[1].tags, vec!["malware"]);
    }

    #[test]
    fn scan_file_finds_matches() {
        let rules = vec![YaraRule {
            name: "test_rule".into(),
            tags: vec![],
            strings: vec!["evil_payload".into()],
            source: PathBuf::from("test.yar"),
        }];
        let hits = scan_file(&rules, b"this file contains evil_payload here");
        assert_eq!(hits, vec!["test_rule"]);
    }

    #[test]
    fn scan_file_no_false_positives() {
        let rules = vec![YaraRule {
            name: "test_rule".into(),
            tags: vec![],
            strings: vec!["definitely_not_present".into()],
            source: PathBuf::from("test.yar"),
        }];
        let hits = scan_file(&rules, b"this file is clean");
        assert!(hits.is_empty());
    }
}
