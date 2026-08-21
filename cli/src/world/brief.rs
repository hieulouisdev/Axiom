//! v1.7.0 — Daily intelligence brief composer.
//!
//! Combines the other world modules into a single human-readable brief
//! that the agent can paste into a system prompt or the user can read
//! directly. Inspired by worldmonitor's "operational picture" but
//! distilled to plain text.

use serde::{Deserialize, Serialize};

use super::finance::{FinanceQuote, render_quotes};
use super::geopolitics::{CountryRisk, render_country_risks};
use super::news::{NewsBrief, render_brief_block};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefItem {
    pub kind: String, // "headline" | "market" | "risk" | "footer"
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyBrief {
    pub title: String,
    pub generated_at_ms: i64,
    pub items: Vec<BriefItem>,
    pub as_text: String,
}

pub struct BriefComposer;

impl BriefComposer {
    /// Compose a daily brief from the three input slices.
    pub fn compose(
        title: &str,
        news: &[NewsBrief],
        quotes: &[FinanceQuote],
        risks: &[CountryRisk],
    ) -> DailyBrief {
        let generated_at_ms = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let mut items = Vec::new();
        let mut as_text = String::new();
        let header = format!("# {}\n_Generated {}_\n", title, generated_at_ms);
        as_text.push_str(&header);

        if !news.is_empty() {
            let block = render_brief_block(news);
            items.push(BriefItem { kind: "headline".into(), text: block.clone() });
            as_text.push_str(&block);
            as_text.push('\n');
        }
        if !quotes.is_empty() {
            let block = render_quotes(quotes);
            let with_header = format!("## Markets\n{}", block);
            items.push(BriefItem { kind: "market".into(), text: with_header.clone() });
            as_text.push_str(&with_header);
            as_text.push('\n');
        }
        if !risks.is_empty() {
            let block = render_country_risks(risks);
            items.push(BriefItem { kind: "risk".into(), text: block.clone() });
            as_text.push_str(&block);
            as_text.push('\n');
        }
        let footer = format!(
            "## End of brief\n_Sources: Aegis AI World Intelligence v1.7_\n"
        );
        items.push(BriefItem { kind: "footer".into(), text: footer.clone() });
        as_text.push_str(&footer);

        DailyBrief {
            title: title.into(),
            generated_at_ms,
            items,
            as_text,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::news::NewsCategory;
    use super::super::geopolitics::{RiskLevel, RiskFactor};

    #[test]
    fn compose_renders_all_sections() {
        let news = vec![NewsBrief {
            title: "Breaking: summit announced".into(),
            link: "x".into(),
            summary: "World leaders convene.".into(),
            category: NewsCategory::World,
            feed_title: "Reuters".into(),
            published_at_ms: 0,
            salience: 0.9,
        }];
        let quotes = vec![FinanceQuote {
            symbol: "AAPL".into(), price: 225.0, currency: "USD".into(),
            change_pct: Some(1.2), source: "Stooq".into(), fetched_at_ms: 0,
        }];
        let risks = vec![CountryRisk {
            iso3: "RU".into(), name: "Russia".into(), score: 75.0,
            level: RiskLevel::High, factors: vec![RiskFactor {
                label: "news_volume".into(), weight: 0.3, raw_value: 80.0, contribution: 24.0
            }], computed_at_ms: 0,
        }];
        let b = BriefComposer::compose("Daily Brief", &news, &quotes, &risks);
        assert!(b.as_text.contains("Daily Brief"));
        assert!(b.as_text.contains("Markets"));
        assert!(b.as_text.contains("Country instability"));
        assert!(b.as_text.contains("Russia"));
        assert_eq!(b.items.len(), 4); // headline + market + risk + footer
    }
}
