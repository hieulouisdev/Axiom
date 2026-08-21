//! v1.7.0 — Geopolitical risk scoring (Country Instability Index).
//!
//! Inspired by worldmonitor's CII v8. Computes a `0..100` instability
//! score for a country by combining weighted signals from news volume,
//! keyword frequency ("protest", "coup", "sanction", "war"), and an
//! optional baseline.
//!
//! ## Inputs
//!
//! - `news_volume`     — number of news items in the past 24h mentioning the country
//! - `negative_ratio`  — fraction of those items with high-signal keywords
//! - `disaster_count`  — count of disaster events in the past 24h
//! - `market_stress`   — `0..1` (e.g. equity index drop >5% → 1.0)
//!
//! ## Output
//!
//! - [`CountryRisk`]   — combined score, risk level, contributing factors

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Stable,
    Watch,
    Elevated,
    High,
    Critical,
}

impl RiskLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            RiskLevel::Stable => "stable",
            RiskLevel::Watch => "watch",
            RiskLevel::Elevated => "elevated",
            RiskLevel::High => "high",
            RiskLevel::Critical => "critical",
        }
    }
    pub fn from_score(score: f64) -> Self {
        if score < 20.0 {
            RiskLevel::Stable
        } else if score < 40.0 {
            RiskLevel::Watch
        } else if score < 60.0 {
            RiskLevel::Elevated
        } else if score < 80.0 {
            RiskLevel::High
        } else {
            RiskLevel::Critical
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryRisk {
    pub iso3: String,
    pub name: String,
    pub score: f64,
    pub level: RiskLevel,
    pub factors: Vec<RiskFactor>,
    pub computed_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub label: String,
    pub weight: f64,
    pub raw_value: f64,
    pub contribution: f64,
}

/// Inputs to the CII computation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstabilityIndex {
    pub news_volume: u32,
    pub negative_ratio: f64,
    pub disaster_count: u32,
    pub market_stress: f64,
}

impl InstabilityIndex {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Compute the instability score for a country.
///
/// The weights mirror worldmonitor's CII v8 design but with a simpler,
/// deterministic formula (no ML model needed):
///
/// - news_volume (capped at 100, scaled to 0..30)
/// - negative_ratio (0..1 → 0..40)
/// - disaster_count (capped at 10, scaled to 0..20)
/// - market_stress (0..1 → 0..10)
pub fn score_country(iso3: &str, name: &str, idx: &InstabilityIndex) -> CountryRisk {
    let news_v = (idx.news_volume as f64).min(100.0) / 100.0 * 30.0;
    let neg_r = idx.negative_ratio.clamp(0.0, 1.0) * 40.0;
    let dis = (idx.disaster_count as f64).min(10.0) / 10.0 * 20.0;
    let mkt = idx.market_stress.clamp(0.0, 1.0) * 10.0;
    let score = (news_v + neg_r + dis + mkt).round().min(100.0);
    let level = RiskLevel::from_score(score);
    let factors = vec![
        RiskFactor { label: "news_volume".into(),    weight: 0.30, raw_value: idx.news_volume as f64, contribution: news_v },
        RiskFactor { label: "negative_ratio".into(), weight: 0.40, raw_value: idx.negative_ratio,      contribution: neg_r  },
        RiskFactor { label: "disaster_count".into(), weight: 0.20, raw_value: idx.disaster_count as f64, contribution: dis   },
        RiskFactor { label: "market_stress".into(),  weight: 0.10, raw_value: idx.market_stress,      contribution: mkt   },
    ];
    CountryRisk {
        iso3: iso3.to_string(),
        name: name.to_string(),
        score,
        level,
        factors,
        computed_at_ms: time::OffsetDateTime::now_utc().unix_timestamp() * 1000,
    }
}

/// Compute instability for a list of countries.
pub fn score_countries(items: &[(String, String, InstabilityIndex)]) -> Vec<CountryRisk> {
    let mut out: Vec<_> = items
        .iter()
        .map(|(iso3, name, idx)| score_country(iso3, name, idx))
        .collect();
    out.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    out
}

/// Render a list of country risks as a text block.
pub fn render_country_risks(items: &[CountryRisk]) -> String {
    if items.is_empty() {
        return "(no risk data)".into();
    }
    let mut out = String::with_capacity(512);
    out.push_str("## Country instability index\n");
    for c in items {
        out.push_str(&format!(
            "- **{}** ({}) — score {:.1}/100 [{}]\n",
            c.name, c.iso3, c.score, c.level.as_str()
        ));
        for f in &c.factors {
            out.push_str(&format!(
                "    - {}: {:.2} → +{:.1}\n",
                f.label, f.raw_value, f.contribution
            ));
        }
    }
    out
}

/// Classify a news item title for negative keyword presence.
pub fn classify_news_sentiment(title: &str) -> bool {
    let l = title.to_lowercase();
    const NEG: &[&str] = &[
        "protest", "riot", "coup", "war", "invasion", "attack", "strike",
        "sanction", "crash", "crisis", "evacuat", "casualt", "killed",
        "wounded", "explos", "missile", "ceasefire violation", "default",
        "emergency", "collapse",
    ];
    NEG.iter().any(|k| l.contains(k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_extremes() {
        let calm = InstabilityIndex { news_volume: 2, negative_ratio: 0.05, disaster_count: 0, market_stress: 0.0 };
        let r_calm = score_country("VN", "Vietnam", &calm);
        assert!(r_calm.score < 10.0);
        assert_eq!(r_calm.level, RiskLevel::Stable);

        let crisis = InstabilityIndex { news_volume: 100, negative_ratio: 0.9, disaster_count: 5, market_stress: 1.0 };
        let r_crisis = score_country("XX", "Crisisland", &crisis);
        assert!(r_crisis.score >= 90.0);
        assert_eq!(r_crisis.level, RiskLevel::Critical);
    }

    #[test]
    fn score_sort_desc() {
        let items = vec![
            ("US".into(), "United States".into(), InstabilityIndex { news_volume: 20, negative_ratio: 0.3, disaster_count: 0, market_stress: 0.2 }),
            ("RU".into(), "Russia".into(),       InstabilityIndex { news_volume: 80, negative_ratio: 0.8, disaster_count: 2, market_stress: 0.5 }),
            ("JP".into(), "Japan".into(),        InstabilityIndex { news_volume: 5,  negative_ratio: 0.1, disaster_count: 0, market_stress: 0.0 }),
        ];
        let r = score_countries(&items);
        assert_eq!(r[0].iso3, "RU");
        assert_eq!(r[r.len() - 1].iso3, "JP");
    }

    #[test]
    fn sentiment_classifier() {
        assert!(classify_news_sentiment("Massive protest erupts in capital"));
        assert!(classify_news_sentiment("Bank crash causes emergency"));
        assert!(!classify_news_sentiment("Local team wins championship"));
    }
}
