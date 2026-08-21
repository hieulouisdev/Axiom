//! v1.7.0 — Wiki: structured knowledge pages with a link graph.
//!
//! Inspired by TencentDB Agent Memory's "Wiki" layer, this module turns
//! loose notes / docs into a navigable knowledge base. Each page has:
//!
//! - a unique `slug` (e.g. `auth-module`, `release-checklist`)
//! - a `title`, `body` (Markdown), and a list of `tags`
//! - outgoing links to other wiki pages (bidirectional)
//! - a `source` field tracking where the page came from (manual, doc
//!   import, agent-generated)
//!
//! The link graph enables "what's related to X?" queries the agent can
//! use to ground its answers without re-reading every file.

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::store::SharedConn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiPage {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub source: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WikiLink {
    pub from_slug: String,
    pub to_slug: String,
    pub label: Option<String>,
}

pub struct Wiki {
    conn: SharedConn,
}

impl Wiki {
    pub fn new(conn: SharedConn) -> Self {
        Self { conn }
    }

    pub fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS wiki_pages (
                id            INTEGER PRIMARY KEY AUTOINCREMENT,
                slug          TEXT NOT NULL UNIQUE,
                title         TEXT NOT NULL,
                body          TEXT NOT NULL DEFAULT '',
                tags          TEXT NOT NULL DEFAULT '',
                source        TEXT,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_wiki_tags ON wiki_pages(tags);

            CREATE TABLE IF NOT EXISTS wiki_links (
                from_slug TEXT NOT NULL,
                to_slug   TEXT NOT NULL,
                label     TEXT,
                PRIMARY KEY (from_slug, to_slug, label)
            );
            CREATE INDEX IF NOT EXISTS idx_wiki_links_from ON wiki_links(from_slug);
            CREATE INDEX IF NOT EXISTS idx_wiki_links_to   ON wiki_links(to_slug);
            "#,
        )?;
        Ok(())
    }

    pub fn upsert_page(
        &self,
        slug: &str,
        title: &str,
        body: &str,
        tags: &[String],
        source: Option<&str>,
    ) -> Result<i64> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let tags_csv = tags.join(",");
        let conn = self.conn.lock();
        conn.execute(
            r#"INSERT INTO wiki_pages (slug, title, body, tags, source, created_at_ms, updated_at_ms)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
               ON CONFLICT(slug) DO UPDATE SET
                   title = excluded.title,
                   body  = excluded.body,
                   tags  = excluded.tags,
                   source = COALESCE(excluded.source, wiki_pages.source),
                   updated_at_ms = excluded.updated_at_ms"#,
            params![slug, title, body, tags_csv, source, now as i64],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_page(&self, slug: &str) -> Result<Option<WikiPage>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"SELECT id, slug, title, body, tags, source, created_at_ms, updated_at_ms
                 FROM wiki_pages WHERE slug = ?1"#,
        )?;
        let mut rows = stmt.query_map(params![slug], |row| {
            let tags_csv: String = row.get(4)?;
            let tags = tags_csv
                .split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            Ok(WikiPage {
                id: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
                body: row.get(3)?,
                tags,
                source: row.get(5)?,
                created_at_ms: row.get(6)?,
                updated_at_ms: row.get(7)?,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r?)),
            None => Ok(None),
        }
    }

    pub fn list_pages(&self) -> Result<Vec<WikiPage>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"SELECT id, slug, title, body, tags, source, created_at_ms, updated_at_ms
                 FROM wiki_pages ORDER BY updated_at_ms DESC"#,
        )?;
        let rows = stmt.query_map([], |row| {
            let tags_csv: String = row.get(4)?;
            let tags = tags_csv
                .split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            Ok(WikiPage {
                id: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
                body: row.get(3)?,
                tags,
                source: row.get(5)?,
                created_at_ms: row.get(6)?,
                updated_at_ms: row.get(7)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn delete_page(&self, slug: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM wiki_pages WHERE slug = ?1", params![slug])?;
        conn.execute(
            "DELETE FROM wiki_links WHERE from_slug = ?1 OR to_slug = ?1",
            params![slug],
        )?;
        Ok(())
    }

    pub fn add_link(&self, from: &str, to: &str, label: Option<&str>) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR IGNORE INTO wiki_links (from_slug, to_slug, label) VALUES (?1, ?2, ?3)",
            params![from, to, label],
        )?;
        Ok(())
    }

    pub fn remove_link(&self, from: &str, to: &str, label: Option<&str>) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM wiki_links WHERE from_slug = ?1 AND to_slug = ?2 AND COALESCE(label,'') = COALESCE(?3,'')",
            params![from, to, label],
        )?;
        Ok(())
    }

    /// Outgoing links from a page.
    pub fn links_from(&self, slug: &str) -> Result<Vec<WikiLink>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT from_slug, to_slug, label FROM wiki_links WHERE from_slug = ?1",
        )?;
        let rows = stmt.query_map(params![slug], |row| {
            Ok(WikiLink {
                from_slug: row.get(0)?,
                to_slug: row.get(1)?,
                label: row.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Incoming links to a page (backlinks).
    pub fn links_to(&self, slug: &str) -> Result<Vec<WikiLink>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT from_slug, to_slug, label FROM wiki_links WHERE to_slug = ?1",
        )?;
        let rows = stmt.query_map(params![slug], |row| {
            Ok(WikiLink {
                from_slug: row.get(0)?,
                to_slug: row.get(1)?,
                label: row.get(2)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Full-text search across title + body (LIKE-based; good enough for
    /// small/medium knowledge bases).
    pub fn search(&self, query: &str) -> Result<Vec<WikiPage>> {
        let like = format!("%{}%", query);
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"SELECT id, slug, title, body, tags, source, created_at_ms, updated_at_ms
                 FROM wiki_pages
                WHERE title LIKE ?1 OR body LIKE ?1 OR tags LIKE ?1
                ORDER BY updated_at_ms DESC"#,
        )?;
        let rows = stmt.query_map(params![like], |row| {
            let tags_csv: String = row.get(4)?;
            let tags = tags_csv
                .split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            Ok(WikiPage {
                id: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
                body: row.get(3)?,
                tags,
                source: row.get(5)?,
                created_at_ms: row.get(6)?,
                updated_at_ms: row.get(7)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn count(&self) -> Result<i64> {
        let conn = self.conn.lock();
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM wiki_pages", [], |row| row.get(0))?;
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use parking_lot::Mutex;

    fn open() -> Wiki {
        let conn = Arc::new(Mutex::new(rusqlite::Connection::open_in_memory().unwrap()));
        let w = Wiki::new(conn);
        w.migrate().unwrap();
        w
    }

    #[test]
    fn page_upsert_and_links() {
        let w = open();
        w.upsert_page("auth", "Auth Module", "Handles JWT", &["security".into(), "api".into()], Some("manual")).unwrap();
        w.upsert_page("jwt", "JWT", "JSON Web Tokens", &["auth".into()], None).unwrap();
        w.add_link("auth", "jwt", Some("uses")).unwrap();
        let from = w.links_from("auth").unwrap();
        assert_eq!(from.len(), 1);
        assert_eq!(from[0].to_slug, "jwt");
        let to = w.links_to("jwt").unwrap();
        assert_eq!(to.len(), 1);
        assert_eq!(to[0].from_slug, "auth");
    }

    #[test]
    fn search_finds_in_body() {
        let w = open();
        w.upsert_page("a", "A", "Rust is great", &[], None).unwrap();
        let r = w.search("rust").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].slug, "a");
    }
}
