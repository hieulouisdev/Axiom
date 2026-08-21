//! Wiki store — slim version for the CLI.

use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

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
            CREATE TABLE IF NOT EXISTS wiki_links (
                from_slug TEXT NOT NULL,
                to_slug   TEXT NOT NULL,
                label     TEXT,
                PRIMARY KEY (from_slug, to_slug, label)
            );
            "#,
        )?;
        Ok(())
    }

    pub fn upsert_page(&self, slug: &str, title: &str, body: &str, tags: &[String], source: Option<&str>) -> Result<i64> {
        let now = chrono::Utc::now().timestamp_millis();
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
            params![slug, title, body, tags_csv, source, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn list_pages(&self) -> Result<Vec<WikiPage>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, slug, title, body, tags, source, created_at_ms, updated_at_ms FROM wiki_pages ORDER BY updated_at_ms DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            let tags_csv: String = row.get(4)?;
            let tags = tags_csv.split(',').filter(|s| !s.is_empty()).map(str::to_string).collect();
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

    pub fn get_page(&self, slug: &str) -> Result<Option<WikiPage>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, slug, title, body, tags, source, created_at_ms, updated_at_ms FROM wiki_pages WHERE slug = ?1",
        )?;
        let mut rows = stmt.query_map(params![slug], |row| {
            let tags_csv: String = row.get(4)?;
            let tags = tags_csv.split(',').filter(|s| !s.is_empty()).map(str::to_string).collect();
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

    pub fn delete_page(&self, slug: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM wiki_pages WHERE slug = ?1", params![slug])?;
        conn.execute("DELETE FROM wiki_links WHERE from_slug = ?1 OR to_slug = ?1", params![slug])?;
        Ok(())
    }

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
            let tags = tags_csv.split(',').filter(|s| !s.is_empty()).map(str::to_string).collect();
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
}
