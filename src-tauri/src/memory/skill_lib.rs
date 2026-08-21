//! v1.7.0 — Skill Library with versions (inspired by TencentDB Agent Memory).
//!
//! A Skill is a reusable, versioned execution recipe: prompt template +
//! resource files + trigger conditions + execution steps + validation
//! rules. Unlike the existing flat `ai::skills` registry (which is a
//! hardcoded enum), this module lets the agent and the user **persist,
//! version, and share** skills in SQLite.
//!
//! ## Lifecycle
//!
//! ```text
//! draft → review → published → deprecated → archived
//! ```
//!
//! Only `published` skills are eligible for runtime injection. The
//! frontend can list, edit, and promote drafts.
//!
//! ## Visibility
//!
//! - `private` — only the owner (default for user-authored skills)
//! - `team`    — shared within a team (multi-agent scenario)
//! - `public`  — anyone (used for built-in skills)

use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::error::Result;

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

/// A single trigger condition for a skill.
///
/// Triggers fire when the user message matches a keyword OR is classified
/// into one of the listed intent tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTrigger {
    /// Lower-cased keywords; any substring match fires the trigger.
    pub keywords: Vec<String>,
    /// Optional intent tags (e.g. "code_review", "debug", "translate").
    pub intents: Vec<String>,
}

/// A single execution step. Steps are rendered into the system prompt as
/// an ordered checklist the agent must follow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStep {
    pub description: String,
    /// Optional tool name to invoke (e.g. "shell", "web_search", "memory_search").
    pub tool: Option<String>,
}

/// A versioned skill document.
///
/// Each `publish` creates a new `SkillVersion` row with a monotonic
/// `version` number; only the latest `published` version is loaded into
/// the agent context by default, but historical versions are queryable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: i64,
    pub slug: String,            // e.g. "code-reviewer"
    pub name: String,            // e.g. "Code Reviewer"
    pub description: String,
    pub visibility: Visibility,
    pub owner: String,
    pub current_status: SkillStatus,
    pub current_version: i64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// A specific version snapshot of a skill's content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVersion {
    pub id: i64,
    pub skill_id: i64,
    pub version: i64,
    pub system_prompt: String,
    pub trigger: SkillTrigger,
    pub steps: Vec<SkillStep>,
    pub validation_rules: Vec<String>,
    pub resources: Vec<String>, // paths/URLs
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
                validation_json   TEXT NOT NULL,
                resources_json    TEXT NOT NULL,
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

    /// Create a new skill in `Draft` status, version 0.
    pub fn create_skill(
        &self,
        slug: &str,
        name: &str,
        description: &str,
        visibility: Visibility,
        owner: &str,
    ) -> Result<i64> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let conn = self.conn.lock();
        conn.execute(
            r#"INSERT INTO skills
                 (slug, name, description, visibility, owner, current_status, current_version,
                  created_at_ms, updated_at_ms)
               VALUES (?1, ?2, ?3, ?4, ?5, 'draft', 0, ?6, ?6)"#,
            params![slug, name, description, visibility.as_str(), owner, now as i64],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Save a new version of a skill. Returns the new version number.
    /// The new version is saved in `Draft` status; the skill's
    /// `current_version` is bumped only when `publish_version()` is called.
    pub fn save_version(
        &self,
        skill_id: i64,
        system_prompt: &str,
        trigger: &SkillTrigger,
        steps: &[SkillStep],
        validation_rules: &[String],
        resources: &[String],
        changelog: Option<&str>,
    ) -> Result<i64> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let trigger_json = serde_json::to_string(trigger)
            .map_err(|e| crate::error::Error::Other(format!("trigger json: {e}")))?;
        let steps_json = serde_json::to_string(steps)
            .map_err(|e| crate::error::Error::Other(format!("steps json: {e}")))?;
        let validation_json = serde_json::to_string(validation_rules)
            .map_err(|e| crate::error::Error::Other(format!("validation json: {e}")))?;
        let resources_json = serde_json::to_string(resources)
            .map_err(|e| crate::error::Error::Other(format!("resources json: {e}")))?;
        let conn = self.conn.lock();
        // next version = max(version) + 1, defaulting to 1
        let next_version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) + 1 FROM skill_versions WHERE skill_id = ?1",
                params![skill_id],
                |row| row.get(0),
            )
            .unwrap_or(1);
        conn.execute(
            r#"INSERT INTO skill_versions
                 (skill_id, version, system_prompt, trigger_json, steps_json,
                  validation_json, resources_json, status, changelog, created_at_ms)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'draft', ?8, ?9)"#,
            params![
                skill_id,
                next_version,
                system_prompt,
                trigger_json,
                steps_json,
                validation_json,
                resources_json,
                changelog,
                now as i64,
            ],
        )?;
        Ok(next_version)
    }

    /// Promote a saved draft version to `Published`. This bumps the skill's
    /// `current_version` and `current_status` atomically; any previously
    /// published version is moved to `Deprecated`.
    pub fn publish_version(&self, skill_id: i64, version: i64) -> Result<()> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        tx.execute(
            "UPDATE skill_versions SET status = 'deprecated'
              WHERE skill_id = ?1 AND status = 'published'",
            params![skill_id],
        )?;
        tx.execute(
            "UPDATE skill_versions SET status = 'published' WHERE skill_id = ?1 AND version = ?2",
            params![skill_id, version],
        )?;
        tx.execute(
            r#"UPDATE skills
                  SET current_version = ?2,
                      current_status  = 'published',
                      updated_at_ms   = ?3
                WHERE id = ?1"#,
            params![skill_id, version, now as i64],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn list_skills(&self, include_archived: bool) -> Result<Vec<Skill>> {
        let conn = self.conn.lock();
        let sql = if include_archived {
            r#"SELECT id, slug, name, description, visibility, owner, current_status,
                      current_version, created_at_ms, updated_at_ms
                 FROM skills
                ORDER BY updated_at_ms DESC"#
        } else {
            r#"SELECT id, slug, name, description, visibility, owner, current_status,
                      current_version, created_at_ms, updated_at_ms
                 FROM skills
                WHERE current_status != 'archived'
                ORDER BY updated_at_ms DESC"#
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
            r#"SELECT id, slug, name, description, visibility, owner, current_status,
                      current_version, created_at_ms, updated_at_ms
                 FROM skills
                WHERE slug = ?1"#,
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

    /// Load the currently-published version of a skill. Returns `None` if
    /// the skill has no published version.
    pub fn load_published_version(&self, skill_id: i64) -> Result<Option<SkillVersion>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"SELECT id, skill_id, version, system_prompt, trigger_json, steps_json,
                      validation_json, resources_json, status, changelog, created_at_ms
                 FROM skill_versions
                WHERE skill_id = ?1 AND status = 'published'
                ORDER BY version DESC
                LIMIT 1"#,
        )?;
        let mut rows = stmt.query_map(params![skill_id], |row| {
            let trigger_json: String = row.get(4)?;
            let steps_json: String = row.get(5)?;
            let validation_json: String = row.get(6)?;
            let resources_json: String = row.get(7)?;
            let status_str: String = row.get(8)?;
            let trigger: SkillTrigger = serde_json::from_str(&trigger_json).unwrap_or(SkillTrigger {
                keywords: vec![],
                intents: vec![],
            });
            let steps: Vec<SkillStep> = serde_json::from_str(&steps_json).unwrap_or_default();
            let validation: Vec<String> = serde_json::from_str(&validation_json).unwrap_or_default();
            let resources: Vec<String> = serde_json::from_str(&resources_json).unwrap_or_default();
            Ok(SkillVersion {
                id: row.get(0)?,
                skill_id: row.get(1)?,
                version: row.get(2)?,
                system_prompt: row.get(3)?,
                trigger,
                steps,
                validation_rules: validation,
                resources,
                status: SkillStatus::parse(&status_str).unwrap_or(SkillStatus::Draft),
                changelog: row.get(9)?,
                created_at_ms: row.get(10)?,
            })
        })?;
        match rows.next() {
            Some(r) => Ok(Some(r?)),
            None => Ok(None),
        }
    }

    pub fn list_versions(&self, skill_id: i64) -> Result<Vec<SkillVersion>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            r#"SELECT id, skill_id, version, system_prompt, trigger_json, steps_json,
                      validation_json, resources_json, status, changelog, created_at_ms
                 FROM skill_versions
                WHERE skill_id = ?1
                ORDER BY version DESC"#,
        )?;
        let rows = stmt.query_map(params![skill_id], |row| {
            let trigger_json: String = row.get(4)?;
            let steps_json: String = row.get(5)?;
            let validation_json: String = row.get(6)?;
            let resources_json: String = row.get(7)?;
            let status_str: String = row.get(8)?;
            let trigger: SkillTrigger = serde_json::from_str(&trigger_json).unwrap_or(SkillTrigger {
                keywords: vec![],
                intents: vec![],
            });
            let steps: Vec<SkillStep> = serde_json::from_str(&steps_json).unwrap_or_default();
            let validation: Vec<String> = serde_json::from_str(&validation_json).unwrap_or_default();
            let resources: Vec<String> = serde_json::from_str(&resources_json).unwrap_or_default();
            Ok(SkillVersion {
                id: row.get(0)?,
                skill_id: row.get(1)?,
                version: row.get(2)?,
                system_prompt: row.get(3)?,
                trigger,
                steps,
                validation_rules: validation,
                resources,
                status: SkillStatus::parse(&status_str).unwrap_or(SkillStatus::Draft),
                changelog: row.get(9)?,
                created_at_ms: row.get(10)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Find skills whose trigger matches the given message. Returns slugs
    /// of all published skills that should fire, sorted by specificity
    /// (longest keyword match first).
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
        Ok(matched.into_iter().map(|(_, slug)| slug).collect())
    }

    pub fn delete_skill(&self, skill_id: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM skills WHERE id = ?1", params![skill_id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use parking_lot::Mutex;

    fn open() -> SkillLibrary {
        let conn = Arc::new(Mutex::new(rusqlite::Connection::open_in_memory().unwrap()));
        let s = SkillLibrary::new(conn);
        s.migrate().unwrap();
        s
    }

    #[test]
    fn create_publish_version() {
        let s = open();
        let id = s.create_skill("code-reviewer", "Code Reviewer", "Reviews Rust code", Visibility::Public, "default").unwrap();
        let v1 = s.save_version(
            id,
            "You are a senior code reviewer.",
            &SkillTrigger { keywords: vec!["review".into(), "code review".into()], intents: vec!["code_review".into()] },
            &[SkillStep { description: "Read the diff".into(), tool: Some("file_read".into()) },
              SkillStep { description: "List concerns".into(), tool: None }],
            &["Must not change public API".into()],
            &[],
            Some("initial"),
        ).unwrap();
        assert_eq!(v1, 1);
        s.publish_version(id, v1).unwrap();
        let v = s.load_published_version(id).unwrap().unwrap();
        assert_eq!(v.version, 1);
        assert_eq!(v.steps.len(), 2);
        assert_eq!(v.trigger.keywords.len(), 2);

        // a new draft shouldn't displace the published one
        let v2 = s.save_version(id, "v2", &SkillTrigger { keywords: vec![], intents: vec![] }, &[], &[], &[], None).unwrap();
        assert_eq!(v2, 2);
        let still_v1 = s.load_published_version(id).unwrap().unwrap();
        assert_eq!(still_v1.version, 1);
        s.publish_version(id, v2).unwrap();
        let now_v2 = s.load_published_version(id).unwrap().unwrap();
        assert_eq!(now_v2.version, 2);
    }

    #[test]
    fn trigger_matching() {
        let s = open();
        let id = s.create_skill("rust-expert", "Rust Expert", "", Visibility::Public, "default").unwrap();
        let _ = s.save_version(
            id,
            "You are a Rust expert.",
            &SkillTrigger { keywords: vec!["rust".into(), "cargo".into()], intents: vec![] },
            &[],
            &[],
            &[],
            None,
        ).unwrap();
        s.publish_version(id, 1).unwrap();
        let m = s.match_triggers("How do I fix this Rust lifetime error?").unwrap();
        assert_eq!(m, vec!["rust-expert".to_string()]);
    }
}
