//! Skill library — slim version for the CLI.

use anyhow::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::store::SharedConn;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillStatus {
    Draft,
    Review,
    Published,
    Deprecated,
    Archived,
}

impl SkillStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            SkillStatus::Draft => "draft",
            SkillStatus::Review => "review",
            SkillStatus::Published => "published",
            SkillStatus::Deprecated => "deprecated",
            SkillStatus::Archived => "archived",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "draft" => SkillStatus::Draft,
            "review" => SkillStatus::Review,
            "published" => SkillStatus::Published,
            "deprecated" => SkillStatus::Deprecated,
            "archived" => SkillStatus::Archived,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Private,
    Team,
    Public,
}

impl Visibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Visibility::Private => "private",
            Visibility::Team => "team",
            Visibility::Public => "public",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "private" => Visibility::Private,
            "team" => Visibility::Team,
            "public" => Visibility::Public,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTrigger {
    pub keywords: Vec<String>,
    pub intents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStep {
    pub description: String,
    pub tool: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub visibility: Visibility,
    pub owner: String,
    pub current_status: SkillStatus,
    pub current_version: i64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    pub id: i64,
    pub skill_id: i64,
    pub version: i64,
    pub system_prompt: String,
    pub trigger: SkillTrigger,
    pub steps: Vec<SkillStep>,
    pub status: SkillStatus,
    pub changelog: Option<String>,
    pub created_at_ms: i64,
}

pub struct SkillLibrary {
    conn: SharedConn,
}

impl SkillLibrary {
    pub fn new(conn: SharedConn) -> Self {
        Self { conn }
    }

    pub fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS skills (
                id               INTEGER PRIMARY KEY AUTOINCREMENT,
                slug             TEXT NOT NULL UNIQUE,
                name             TEXT NOT NULL,
                description      TEXT NOT NULL,
                visibility       TEXT NOT NULL DEFAULT 'private',
                owner            TEXT NOT NULL DEFAULT 'default',
                current_status   TEXT NOT NULL DEFAULT 'draft',
                current_version  INTEGER NOT NULL DEFAULT 0,
                created_at_ms    INTEGER NOT NULL,
                updated_at_ms    INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS skill_versions (
                id                INTEGER PRIMARY KEY AUTOINCREMENT,
                skill_id          INTEGER NOT NULL,
                version           INTEGER NOT NULL,
                system_prompt     TEXT NOT NULL,
                trigger_json      TEXT NOT NULL,
                steps_json        TEXT NOT NULL,
                status            TEXT NOT NULL DEFAULT 'draft',
                changelog         TEXT,
                created_at_ms     INTEGER NOT NULL,
                UNIQUE(skill_id, version),
                FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_skill_versions_skill ON skill_versions(skill_id);
            "#,
        )?;
        Ok(())
    }

    pub fn create_skill(&self, slug: &str, name: &str, description: &str, visibility: Visibility, owner: &str) -> Result<i64> {
        let now = chrono::Utc::now().timestamp_millis();
        let conn = self.conn.lock();
        conn.execute(
            r#"INSERT INTO skills (slug, name, description, visibility, owner, current_status, current_version, created_at_ms, updated_at_ms)
               VALUES (?1, ?2, ?3, ?4, ?5, 'draft', 0, ?6, ?6)"#,
            params![slug, name, description, visibility.as_str(), owner, now],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn save_version(
        &self,
        skill_id: i64,
        system_prompt: &str,
        trigger: &SkillTrigger,
        steps: &[SkillStep],
        changelog: Option<&str>,
    ) -> Result<i64> {
        let now = chrono::Utc::now().timestamp_millis();
        let trigger_json = serde_json::to_string(trigger)?;
        let steps_json = serde_json::to_string(steps)?;
        let conn = self.conn.lock();
        let next_version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) + 1 FROM skill_versions WHERE skill_id = ?1",
                params![skill_id],
                |row| row.get(0),
            )
            .unwrap_or(1);
        conn.execute(
            r#"INSERT INTO skill_versions (skill_id, version, system_prompt, trigger_json, steps_json, status, changelog, created_at_ms)
               VALUES (?1, ?2, ?3, ?4, ?5, 'draft', ?6, ?7)"#,
            params![skill_id, next_version, system_prompt, trigger_json, steps_json, changelog, now],
        )?;
        Ok(next_version)
    }

    pub fn publish_version(&self, skill_id: i64, version: i64) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        tx.execute(
            "UPDATE skill_versions SET status = 'deprecated' WHERE skill_id = ?1 AND status = 'published'",
            params![skill_id],
        )?;
        tx.execute(
            "UPDATE skill_versions SET status = 'published' WHERE skill_id = ?1 AND version = ?2",
            params![skill_id, version],
        )?;
        tx.execute(
            "UPDATE skills SET current_version = ?2, current_status = 'published', updated_at_ms = ?3 WHERE id = ?1",
            params![skill_id, version, now],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_skills(&self, include_archived: bool) -> Result<Vec<Skill>> {
        let conn = self.conn.lock();
        let sql = if include_archived {
            "SELECT id, slug, name, description, visibility, owner, current_status, current_version, created_at_ms, updated_at_ms FROM skills ORDER BY updated_at_ms DESC"
        } else {
            "SELECT id, slug, name, description, visibility, owner, current_status, current_version, created_at_ms, updated_at_ms FROM skills WHERE current_status != 'archived' ORDER BY updated_at_ms DESC"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| {
            let vis_str: String = row.get(4)?;
            let status_str: String = row.get(6)?;
            Ok(Skill {
                id: row.get(0)?,
                slug: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                visibility: Visibility::parse(&vis_str).unwrap_or(Visibility::Private),
                owner: row.get(5)?,
                current_status: SkillStatus::parse(&status_str).unwrap_or(SkillStatus::Draft),
                current_version: row.get(7)?,
                created_at_ms: row.get(8)?,
                updated_at_ms: row.get(9)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn get_skill_by_slug(&self, slug: &str) -> Result<Option<Skill>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, slug, name, description, visibility, owner, current_status, current_version, created_at_ms, updated_at_ms FROM skills WHERE slug = ?1",
        )?;
        let mut rows = stmt.query_map(params![slug], |row| {
            let vis_str: String = row.get(4)?;
            let status_str: String = row.get(6)?;
            Ok(Skill {
                id: row.get(0)?,
                slug: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                visibility: Visibility::parse(&vis_str).unwrap_or(Visibility::Private),
                owner: row.get(5)?,
                current_status: SkillStatus::parse(&status_str).unwrap_or(SkillStatus::Draft),
                current_version: row.get(7)?,
                created_at_ms: row.get(8)?,
                updated_at_ms: row.get(9)?,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r?)),
            None => Ok(None),
        }
    }

    pub fn load_published_version(&self, skill_id: i64) -> Result<Option<SkillVersion>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"SELECT id, skill_id, version, system_prompt, trigger_json, steps_json, status, changelog, created_at_ms
                 FROM skill_versions
                WHERE skill_id = ?1 AND status = 'published'
                ORDER BY version DESC LIMIT 1"#,
        )?;
        let mut rows = stmt.query_map(params![skill_id], |row| {
            let trigger_json: String = row.get(4)?;
            let steps_json: String = row.get(5)?;
            let status_str: String = row.get(6)?;
            let trigger: SkillTrigger = serde_json::from_str(&trigger_json).unwrap_or(SkillTrigger { keywords: vec![], intents: vec![] });
            let steps: Vec<SkillStep> = serde_json::from_str(&steps_json).unwrap_or_default();
            Ok(SkillVersion {
                id: row.get(0)?,
                skill_id: row.get(1)?,
                version: row.get(2)?,
                system_prompt: row.get(3)?,
                trigger,
                steps,
                status: SkillStatus::parse(&status_str).unwrap_or(SkillStatus::Draft),
                changelog: row.get(7)?,
                created_at_ms: row.get(8)?,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r?)),
            None => Ok(None),
        }
    }

    pub fn match_triggers(&self, message: &str) -> Result<Vec<String>> {
        let skills = self.list_skills(false)?;
        let lower = message.to_lowercase();
        let mut matched: Vec<(usize, String)> = Vec::new();
        for s in skills {
            if s.current_status != SkillStatus::Published {
                continue;
            }
            if let Some(v) = self.load_published_version(s.id)? {
                for kw in &v.trigger.keywords {
                    if !kw.is_empty() && lower.contains(&kw.to_lowercase()) {
                        matched.push((kw.len(), s.slug.clone()));
                        break;
                    }
                }
            }
        }
        matched.sort_by(|a, b| b.0.cmp(&a.0));
        Ok(matched.into_iter().map(|(_, s)| s).collect())
    }
}
