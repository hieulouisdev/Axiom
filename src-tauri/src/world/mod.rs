//! v1.7.0 — World Intelligence module.
//!
//! Inspired by [worldmonitor](https://github.com/koala73/worldmonitor), this
//! module gives Aegis AI a real-time view of the world: news, geopolitical
//! events, financial markets, country instability scores, and disaster
//! signals. All in pure Rust, with no external services to run.
//!
//! ## Sub-modules
//!
//! - [`feeds`] — RSS/Atom feed registry + fetcher + normalizer
//! - [`news`] — news brief aggregator with dedup + categorization
//! - [`finance`] — market data (stocks, FX, crypto, commodities)
//! - [`geopolitics`] — country instability index (CII), event correlation
//! - [`brief`] — daily intelligence brief composer

pub mod brief;
pub mod feeds;
pub mod finance;
pub mod geopolitics;
pub mod news;

pub use brief::{DailyBrief, BriefItem, BriefComposer};
pub use feeds::{Feed, FeedRegistry, FeedItem, default_feeds};
pub use finance::{FinanceQuote, MarketSnapshot, fetch_quote, fetch_multi};
pub use geopolitics::{CountryRisk, RiskLevel, InstabilityIndex, score_country};
pub use news::{NewsBrief, NewsCategory, NewsAggregator};
