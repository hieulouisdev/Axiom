//! World intelligence — slim CLI version.

pub mod feeds;
pub mod news;
pub mod finance;
pub mod geopolitics;
pub mod brief;

pub use feeds::{Feed, FeedRegistry, FeedItem, FeedFetcher, default_feeds};
pub use news::{NewsBrief, NewsCategory, NewsAggregator, render_brief_block};
pub use finance::{FinanceQuote, MarketSnapshot, fetch_quote, fetch_multi, fetch_market_snapshot, render_quotes};
pub use geopolitics::{CountryRisk, RiskLevel, InstabilityIndex, score_country, score_countries, render_country_risks};
pub use brief::{DailyBrief, BriefItem, BriefComposer};
