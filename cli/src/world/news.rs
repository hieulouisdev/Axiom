//! v1.7.0 — News aggregator.
//!
//! Pulls items from the [`FeedRegistry`], normalizes them, deduplicates
//! (by URL or GUID), categorizes, and ranks by recency + simple keyword
//! salience. The result is a stream of [`NewsBrief`] structs that the
//! agent can inject into its system prompt or that the UI can render.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::feeds::{FeedFetcher, FeedItem, FeedRegistry, default_feeds};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NewsCategory {
    World,
    Geopolitics,
    Tech,
    Finance,
    Security,
    Science,
    Disaster,
    Other,
}

impl NewsCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            NewsCategory::World => "world",
            NewsCategory::Geopolitics => "geopolitics",
            NewsCategory::Tech => "tech",
            NewsCategory::Finance => "finance",
            NewsCategory::Security => "security",
            NewsCategory::Science => "science",
            NewsCategory::Disaster => "disaster",
            NewsCategory::Other => "other",
        }
    }
    pub fn from_feed_category(s: &str) -> Self {
        match s {
            "world" => NewsCategory::World,
            "geopolitics" => NewsCategory::Geopolitics,
            "tech" => NewsCategory::Tech,
            "finance" => NewsCategory::Finance,
            "security" => NewsCategory::Security,
            "science" => NewsCategory::Science,
            "disaster" => NewsCategory::Disaster,
            _ => NewsCategory::Other,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsBrief {
    pub title: String,
    pub link: String,
    pub summary: String,
    pub category: NewsCategory,
    pub feed_title: String,
    pub published_at_ms: i64,
    pub salience: f64,
}

pub struct NewsAggregator {
    registry: FeedRegistry,
    fetcher: FeedFetcher,
}

impl NewsAggregator {
    pub fn new() -> Self {
        Self {
            registry: default_feeds(),
            fetcher: FeedFetcher::new(),
        }
    }

    pub fn with_registry(registry: FeedRegistry) -> Self {
        Self {
            registry,
            fetcher: FeedFetcher::new(),
        }
    }

    pub fn registry(&self) -> &FeedRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut FeedRegistry {
        &mut self.registry
    }

    /// Fetch every feed in the registry and return a deduplicated,
    /// ranked list of news briefs.
    pub async fn fetch_all(&self, limit: usize) -> Vec<NewsBrief> {
        let feeds = self.registry.list();
        let items = self.fetcher.fetch_all(&feeds, 8).await;
        let mut briefs = self.dedupe_and_rank(items);
        briefs.truncate(limit.max(1));
        briefs
    }

    /// Fetch only feeds matching a category.
    pub async fn fetch_category(&self, category: &str, limit: usize) -> Vec<NewsBrief> {
        let feeds = self.registry.list_by_category(category);
        if feeds.is_empty() {
            return Vec::new();
        }
        let items = self.fetcher.fetch_all(&feeds, 4).await;
        let mut briefs = self.dedupe_and_rank(items);
        briefs.truncate(limit.max(1));
        briefs
    }

    fn dedupe_and_rank(&self, items: Vec<FeedItem>) -> Vec<NewsBrief> {
        let mut seen: HashSet<String> = HashSet::new();
        let mut briefs: Vec<NewsBrief> = Vec::new();
        let now_ms = now_ms();
        for it in items {
            let key = if !it.guid.is_empty() {
                it.guid.clone()
            } else if !it.link.is_empty() {
                it.link.clone()
            } else {
                format!("{}|{}", it.feed_id, it.title)
            };
            if !seen.insert(key) {
                continue;
            }
            let salience = compute_salience(&it, now_ms);
            briefs.push(NewsBrief {
                title: it.title,
                link: it.link,
                summary: it.summary,
                category: NewsCategory::from_feed_category(&it.category),
                feed_title: it.feed_title,
                published_at_ms: it.published_at_ms,
                salience,
            });
        }
        briefs.sort_by(|a, b| b.salience.partial_cmp(&a.salience).unwrap_or(std::cmp::Ordering::Equal));
        briefs
    }
}

impl Default for NewsAggregator {
    fn default() -> Self {
        Self::new()
    }
}

fn now_ms() -> i64 {
    time::OffsetDateTime::now_utc().unix_timestamp() * 1000
}

/// Compute a salience score in `[0.0, 1.0]` for ranking.
///
/// Combines:
/// - **Recency** (exponential decay over 24h, half-life 6h)
/// - **Keyword salience** (presence of high-signal words like
///   "breaking", "urgent", "exclusive", "alert", "warning")
fn compute_salience(item: &FeedItem, now_ms: i64) -> f64 {
    let age_h = if item.published_at_ms > 0 && now_ms > 0 {
        ((now_ms - item.published_at_ms) as f64 / 3_600_000.0).max(0.0)
    } else {
        48.0
    };
    let recency = 0.5_f64.powf(age_h / 6.0);
    let title_lower = item.title.to_lowercase();
    let summary_lower = item.summary.to_lowercase();
    let mut salience = recency;
    let keywords = [
        "breaking", "urgent", "exclusive", "alert", "warning", "critical",
        "emergency", "attack", "ceasefire", "summit", "deal", "leak", "crash",
        "rally", "sanction", "coup", "invasion", "evacuat",
    ];
    for k in keywords {
        if title_lower.contains(k) {
            salience += 0.08;
        } else if summary_lower.contains(k) {
            salience += 0.03;
        }
    }
    salience.min(1.0)
}

/// Filter briefs by a free-text query (title OR summary contains).
pub fn filter_briefs(briefs: &[NewsBrief], query: &str) -> Vec<NewsBrief> {
    if query.trim().is_empty() {
        return briefs.to_vec();
    }
    let q = query.to_lowercase();
    briefs
        .iter()
        .filter(|b| {
            b.title.to_lowercase().contains(&q) || b.summary.to_lowercase().contains(&q)
        })
        .cloned()
        .collect()
}

/// Render a list of briefs as a plain-text block suitable for the agent
/// system prompt.
pub fn render_brief_block(briefs: &[NewsBrief]) -> String {
    if briefs.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(1024);
    out.push_str("## World intelligence brief\n");
    for (i, b) in briefs.iter().enumerate() {
        out.push_str(&format!(
            "{}. [{}] {} ({})\n   {}\n   {}\n",
            i + 1,
            b.category.as_str(),
            b.title,
            b.feed_title,
            b.summary.chars().take(180).collect::<String>(),
            b.link,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::feeds::Feed;

    #[test]
    fn salience_recency_decay() {
        let now = 1_700_000_000_000;
        let item_old = FeedItem {
            feed_id: "x".into(),
            feed_title: "x".into(),
            category: "world".into(),
            title: "Old news".into(),
            link: "x".into(),
            summary: "x".into(),
            published_at_ms: now - 48 * 3_600_000,
            guid: "old".into(),
        };
        let item_new = FeedItem {
            feed_id: "x".into(),
            feed_title: "x".into(),
            category: "world".into(),
            title: "Breaking news".into(),
            link: "y".into(),
            summary: "x".into(),
            published_at_ms: now - 30 * 60_000,
            guid: "new".into(),
        };
        let s_old = compute_salience(&item_old, now);
        let s_new = compute_salience(&item_new, now);
        assert!(s_new > s_old, "newer should outrank older: {s_new} vs {s_old}");
        assert!(s_new > 0.5);
    }

    #[test]
    fn dedupe_by_guid() {
        let items = vec![
            FeedItem { feed_id: "x".into(), feed_title: "x".into(), category: "world".into(), title: "A".into(), link: "l1".into(), summary: "s".into(), published_at_ms: 1, guid: "g1".into() },
            FeedItem { feed_id: "x".into(), feed_title: "x".into(), category: "world".into(), title: "A dup".into(), link: "l1".into(), summary: "s".into(), published_at_ms: 1, guid: "g1".into() },
            FeedItem { feed_id: "x".into(), feed_title: "x".into(), category: "world".into(), title: "B".into(), link: "l2".into(), summary: "s".into(), published_at_ms: 1, guid: "g2".into() },
        ];
        let agg = NewsAggregator::new();
        let briefs = agg.dedupe_and_rank(items);
        assert_eq!(briefs.len(), 2);
    }

    #[test]
    fn render_block_smoke() {
        let briefs = vec![NewsBrief {
            title: "T".into(), link: "L".into(), summary: "S".into(),
            category: NewsCategory::World, feed_title: "F".into(),
            published_at_ms: 0, salience: 0.5
        }];
        let s = render_brief_block(&briefs);
        assert!(s.contains("World intelligence brief"));
        assert!(s.contains("T"));
    }
}
