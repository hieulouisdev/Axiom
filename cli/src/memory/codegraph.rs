//! CodeGraph — slim version for the CLI.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::store::SharedConn;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Interface,
    Class,
    Module,
    Constant,
    Type,
    Variable,
}

impl SymbolKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SymbolKind::Function => "function",
            SymbolKind::Method => "method",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Trait => "trait",
            SymbolKind::Interface => "interface",
            SymbolKind::Class => "class",
            SymbolKind::Module => "module",
            SymbolKind::Constant => "constant",
            SymbolKind::Type => "type",
            SymbolKind::Variable => "variable",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "function" => SymbolKind::Function,
            "method" => SymbolKind::Method,
            "struct" => SymbolKind::Struct,
            "enum" => SymbolKind::Enum,
            "trait" => SymbolKind::Trait,
            "interface" => SymbolKind::Interface,
            "class" => SymbolKind::Class,
            "module" => SymbolKind::Module,
            "constant" => SymbolKind::Constant,
            "type" => SymbolKind::Type,
            "variable" => SymbolKind::Variable,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub id: i64,
    pub repo_id: i64,
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub line: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub id: i64,
    pub root_path: String,
    pub name: String,
    pub language: String,
    pub indexed_at_ms: i64,
    pub symbol_count: i64,
    pub edge_count: i64,
}

pub struct CodeGraph {
    conn: SharedConn,
}

impl CodeGraph {
    pub fn new(conn: SharedConn) -> Self {
        Self { conn }
    }

    pub fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS code_repos (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                root_path     TEXT NOT NULL UNIQUE,
                name          TEXT NOT NULL,
                language      TEXT NOT NULL,
                indexed_at_ms INTEGER NOT NULL,
                symbol_count  INTEGER NOT NULL DEFAULT 0,
                edge_count    INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS code_symbols (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id     INTEGER NOT NULL,
                name        TEXT NOT NULL,
                kind        TEXT NOT NULL,
                file_path   TEXT NOT NULL,
                line        INTEGER NOT NULL,
                UNIQUE(repo_id, name, kind, file_path, line),
                FOREIGN KEY (repo_id) REFERENCES code_repos(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_symbols_name ON code_symbols(name);
            CREATE TABLE IF NOT EXISTS code_edges (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_id    INTEGER NOT NULL,
                caller_id  INTEGER NOT NULL,
                callee_id  INTEGER NOT NULL,
                file_path  TEXT NOT NULL,
                line       INTEGER NOT NULL,
                UNIQUE(repo_id, caller_id, callee_id, file_path, line)
            );
            CREATE INDEX IF NOT EXISTS idx_edges_caller ON code_edges(caller_id);
            CREATE INDEX IF NOT EXISTS idx_edges_callee ON code_edges(callee_id);
            "#,
        )?;
        Ok(())
    }

    pub fn register_repo(&self, root_path: &str, name: &str, language: &str) -> Result<i64> {
        let now = chrono::Utc::now().timestamp_millis();
        let conn = self.conn.lock();
        conn.execute(
            r#"INSERT INTO code_repos (root_path, name, language, indexed_at_ms, symbol_count, edge_count)
               VALUES (?1, ?2, ?3, ?4, 0, 0)
               ON CONFLICT(root_path) DO UPDATE SET
                   name = excluded.name,
                   language = excluded.language,
                   indexed_at_ms = excluded.indexed_at_ms"#,
            params![root_path, name, language, now],
        )?;
        let id: i64 = conn.query_row(
            "SELECT id FROM code_repos WHERE root_path = ?1",
            params![root_path],
            |row| row.get(0),
        )?;
        conn.execute("DELETE FROM code_symbols WHERE repo_id = ?1", params![id])?;
        Ok(id)
    }

    pub fn list_repos(&self) -> Result<Vec<Repo>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, root_path, name, language, indexed_at_ms, symbol_count, edge_count FROM code_repos ORDER BY indexed_at_ms DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Repo {
                id: row.get(0)?,
                root_path: row.get(1)?,
                name: row.get(2)?,
                language: row.get(3)?,
                indexed_at_ms: row.get(4)?,
                symbol_count: row.get(5)?,
                edge_count: row.get(6)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn index_dir(&self, repo_id: i64, root: &Path) -> Result<(usize, usize)> {
        let ext = self.repo_language_ext(repo_id)?;
        let mut symbol_count = 0usize;
        let mut edge_count = 0usize;
        let mut name_to_ids: HashMap<String, Vec<i64>> = HashMap::new();
        let files = collect_files(root, &ext)?;
        let conn = self.conn.lock();
        for file_rel in &files {
            let abs = root.join(file_rel);
            let Ok(content) = std::fs::read_to_string(&abs) else { continue };
            let symbols = parse_symbols(&content, &ext);
            for sym in symbols {
                if sym.name.is_empty() || sym.name.starts_with(|c: char| c.is_ascii_digit()) {
                    continue;
                }
                conn.execute(
                    r#"INSERT INTO code_symbols (repo_id, name, kind, file_path, line)
                       VALUES (?1, ?2, ?3, ?4, ?5)
                       ON CONFLICT(repo_id, name, kind, file_path, line) DO NOTHING"#,
                    params![repo_id, sym.name, sym.kind.as_str(), file_rel, sym.line],
                )?;
                let id: i64 = conn.query_row(
                    r#"SELECT id FROM code_symbols WHERE repo_id = ?1 AND name = ?2 AND kind = ?3 AND file_path = ?4 AND line = ?5"#,
                    params![repo_id, sym.name, sym.kind.as_str(), file_rel, sym.line],
                    |row| row.get(0),
                )?;
                name_to_ids.entry(sym.name.clone()).or_default().push(id);
                symbol_count += 1;
                for (callee_name, callee_line) in find_calls_in_body(&content, sym.line as usize) {
                    if let Some(callee_ids) = name_to_ids.get(&callee_name) {
                        if let Some(&callee_id) = callee_ids.first() {
                            let _ = conn.execute(
                                r#"INSERT INTO code_edges (repo_id, caller_id, callee_id, file_path, line)
                                   VALUES (?1, ?2, ?3, ?4, ?5)
                                   ON CONFLICT(repo_id, caller_id, callee_id, file_path, line) DO NOTHING"#,
                                params![repo_id, id, callee_id, file_rel, callee_line as i64],
                            );
                            edge_count += 1;
                        }
                    }
                }
            }
        }
        let now = chrono::Utc::now().timestamp_millis();
        conn.execute(
            "UPDATE code_repos SET symbol_count = ?2, edge_count = ?3, indexed_at_ms = ?4 WHERE id = ?1",
            params![repo_id, symbol_count as i64, edge_count as i64, now],
        )?;
        Ok((symbol_count, edge_count))
    }

    fn repo_language_ext(&self, repo_id: i64) -> Result<String> {
        let conn = self.conn.lock();
        let lang: String = conn.query_row(
            "SELECT language FROM code_repos WHERE id = ?1",
            params![repo_id],
            |row| row.get(0),
        )?;
        Ok(match lang.as_str() {
            "rust" => "rs".into(),
            "typescript" | "tsx" => "ts".into(),
            "javascript" | "jsx" => "js".into(),
            "python" => "py".into(),
            "go" => "go".into(),
            "c" => "c".into(),
            "cpp" => "cpp".into(),
            "java" => "java".into(),
            _ => "rs".into(),
        })
    }

    pub fn search_symbols(&self, name_query: &str, limit: i64) -> Result<Vec<Symbol>> {
        let like = format!("%{}%", name_query);
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, repo_id, name, kind, file_path, line FROM code_symbols WHERE name LIKE ?1 ORDER BY name ASC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![like, limit], |row| {
            let kind_str: String = row.get(3)?;
            Ok(Symbol {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                name: row.get(2)?,
                kind: SymbolKind::parse(&kind_str).unwrap_or(SymbolKind::Function),
                file_path: row.get(4)?,
                line: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn callers_of(&self, symbol_id: i64) -> Result<Vec<Symbol>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"SELECT s.id, s.repo_id, s.name, s.kind, s.file_path, s.line
                 FROM code_symbols s JOIN code_edges e ON e.caller_id = s.id
                WHERE e.callee_id = ?1 ORDER BY s.name ASC"#,
        )?;
        let rows = stmt.query_map(params![symbol_id], |row| {
            let kind_str: String = row.get(3)?;
            Ok(Symbol {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                name: row.get(2)?,
                kind: SymbolKind::parse(&kind_str).unwrap_or(SymbolKind::Function),
                file_path: row.get(4)?,
                line: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn callees_of(&self, symbol_id: i64) -> Result<Vec<Symbol>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"SELECT s.id, s.repo_id, s.name, s.kind, s.file_path, s.line
                 FROM code_symbols s JOIN code_edges e ON e.callee_id = s.id
                WHERE e.caller_id = ?1 ORDER BY s.name ASC"#,
        )?;
        let rows = stmt.query_map(params![symbol_id], |row| {
            let kind_str: String = row.get(3)?;
            Ok(Symbol {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                name: row.get(2)?,
                kind: SymbolKind::parse(&kind_str).unwrap_or(SymbolKind::Function),
                file_path: row.get(4)?,
                line: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

// ── file walker + symbol parser (same as desktop app) ─────────────────────

struct ParsedSymbol {
    name: String,
    kind: SymbolKind,
    line: i64,
}

fn collect_files(root: &Path, ext: &str) -> Result<Vec<String>> {
    let mut out = Vec::new();
    walk(root, root, ext, &mut out)?;
    Ok(out)
}

fn walk(root: &Path, dir: &Path, ext: &str, out: &mut Vec<String>) -> Result<()> {
    let entries = std::fs::read_dir(dir)?;
    for entry in entries.flatten() {
        let p = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            walk(root, &p, ext, out)?;
        } else if p.extension().and_then(|s| s.to_str()) == Some(ext) {
            if let Ok(rel) = p.strip_prefix(root) {
                out.push(rel.to_string_lossy().to_string());
            }
        }
    }
    Ok(())
}

fn parse_symbols(content: &str, ext: &str) -> Vec<ParsedSymbol> {
    let mut out = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let line_no = (i + 1) as i64;
        if let Some((name, kind)) = match ext {
            "rs" => parse_rust_line(line),
            "ts" | "js" => parse_ts_line(line),
            "py" => parse_py_line(line),
            "go" => parse_go_line(line),
            _ => None,
        } {
            if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
                continue;
            }
            out.push(ParsedSymbol { name, kind, line: line_no });
        }
    }
    out
}

fn parse_rust_line(line: &str) -> Option<(String, SymbolKind)> {
    let t = line.trim_start();
    for (prefix, kind) in [
        ("pub async fn ", SymbolKind::Function),
        ("async fn ", SymbolKind::Function),
        ("pub fn ", SymbolKind::Function),
        ("fn ", SymbolKind::Function),
        ("struct ", SymbolKind::Struct),
        ("enum ", SymbolKind::Enum),
        ("trait ", SymbolKind::Trait),
        ("mod ", SymbolKind::Module),
        ("const ", SymbolKind::Constant),
    ] {
        if let Some(rest) = t.strip_prefix(prefix) {
            return Some((take_ident(rest), kind));
        }
    }
    None
}

fn parse_ts_line(line: &str) -> Option<(String, SymbolKind)> {
    let t = line.trim_start();
    for (prefix, kind) in [
        ("export async function ", SymbolKind::Function),
        ("async function ", SymbolKind::Function),
        ("export function ", SymbolKind::Function),
        ("function ", SymbolKind::Function),
        ("export interface ", SymbolKind::Interface),
        ("interface ", SymbolKind::Interface),
        ("export class ", SymbolKind::Class),
        ("class ", SymbolKind::Class),
        ("type ", SymbolKind::Type),
        ("const ", SymbolKind::Constant),
    ] {
        if let Some(rest) = t.strip_prefix(prefix) {
            return Some((take_ident(rest), kind));
        }
    }
    None
}

fn parse_py_line(line: &str) -> Option<(String, SymbolKind)> {
    let t = line.trim_start();
    if let Some(rest) = t.strip_prefix("async def ") {
        return Some((take_ident(rest), SymbolKind::Function));
    }
    if let Some(rest) = t.strip_prefix("def ") {
        return Some((take_ident(rest), SymbolKind::Function));
    }
    if let Some(rest) = t.strip_prefix("class ") {
        return Some((take_ident(rest), SymbolKind::Class));
    }
    None
}

fn parse_go_line(line: &str) -> Option<(String, SymbolKind)> {
    let t = line.trim_start();
    if let Some(rest) = t.strip_prefix("func ") {
        if rest.starts_with('(') {
            if let Some(end) = rest.find(") ") {
                let after = &rest[end + 2..];
                return Some((take_ident(after), SymbolKind::Method));
            }
        }
        return Some((take_ident(rest), SymbolKind::Function));
    }
    if let Some(rest) = t.strip_prefix("type ") {
        let n = take_ident(rest);
        if !n.is_empty() {
            return Some((n, SymbolKind::Type));
        }
    }
    None
}

fn take_ident(s: &str) -> String {
    s.trim_start().chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect()
}

fn find_calls_in_body(content: &str, start_line: usize) -> Vec<(String, i64)> {
    let mut out = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let end = (start_line + 80).min(lines.len());
    for i in start_line..end {
        let line = lines[i - 1];
        let chars: Vec<char> = line.chars().collect();
        let mut j = 0;
        while j < chars.len() {
            if chars[j].is_alphabetic() || chars[j] == '_' {
                let start = j;
                while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                    j += 1;
                }
                let name: String = chars[start..j].iter().collect();
                let mut k = j;
                while k < chars.len() && chars[k].is_whitespace() {
                    k += 1;
                }
                if k < chars.len() && chars[k] == '(' {
                    if !matches!(name.as_str(), "if"|"for"|"while"|"match"|"return"|"println"|"print"|"eprintln"|"format"|"vec"|"Some"|"Ok"|"Err"|"None"|"let") {
                        out.push((name, i as i64));
                    }
                }
            } else {
                j += 1;
            }
        }
    }
    out
}

#[allow(dead_code)]
fn _unused_pathbuf_marker() -> PathBuf { PathBuf::new() }
