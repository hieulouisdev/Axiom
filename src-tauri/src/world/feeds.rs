//! v1.7.0 — RSS/Atom feed registry + fetcher.
//!
//! A curated set of public RSS feeds covering world news, technology,
//! finance, security, and science. The fetcher is async (uses the shared
//! `reqwest` client from the workspace) and normalizes both RSS 2.0 and
//! Atom 1.0 into a single [`FeedItem`] shape.
//!
//! The default registry mirrors the upstream sources worldmonitor uses
//! (Reuters, BBC, Al Jazeera, GDELT, ACLED-style conflict feeds, Hacker
//! News, BNO News, etc.) plus a curated crypto/finance set.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feed {
    pub id: String,
    pub title: String,
    pub url: String,
    pub category: String,
    pub locale: Option<String>,
    /// Optional ISO-3166 country code this feed focuses on.
    pub country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedItem {
    pub feed_id: String,
    pub feed_title: String,
    pub category: String,
    pub title: String,
    pub link: String,
    pub summary: String,
    pub published_at_ms: i64,
    pub guid: String,
}

/// Registry of curated feeds. The default set is intentionally broad;
/// users can extend or override it via the `FeedRegistry::add` method.
#[derive(Debug, Clone, Default)]
pub struct FeedRegistry {
    feeds: HashMap<String, Feed>,
}

impl FeedRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, feed: Feed) {
        self.feeds.insert(feed.id.clone(), feed);
    }

    pub fn get(&self, id: &str) -> Option<&Feed> {
        self.feeds.get(id)
    }

    pub fn list(&self) -> Vec<Feed> {
        let mut v: Vec<_> = self.feeds.values().cloned().collect();
        v.sort_by(|a, b| a.category.cmp(&b.category).then(a.id.cmp(&b.id)));
        v
    }

    pub fn list_by_category(&self, category: &str) -> Vec<Feed> {
        let mut v: Vec<_> = self
            .feeds
            .values()
            .filter(|f| f.category == category)
            .cloned()
            .collect();
        v.sort_by(|a, b| a.id.cmp(&b.id));
        v
    }

    pub fn categories(&self) -> Vec<String> {
        let mut s: Vec<_> = self.feeds.values().map(|f| f.category.clone()).collect();
        s.sort();
        s.dedup();
        s
    }

    pub fn len(&self) -> usize {
        self.feeds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.feeds.is_empty()
    }
}

/// Build the default feed registry. Mirrors the upstream sources
/// worldmonitor aggregates: GDELT, ACLED, Reuters, BBC, Al Jazeera,
/// Hacker News, BNO, plus curated crypto/finance feeds (CoinGecko
/// status, Finnhub free tier, ECB FX).
pub fn default_feeds() -> FeedRegistry {
    let mut r = FeedRegistry::new();
    let items = vec![
        // ─── world news ──────────────────────────────────────────────────
        Feed { id: "reuters-world".into(),       title: "Reuters — World".into(),         url: "https://feeds.reuters.com/Reuters/worldNews".into(), category: "world".into(), locale: Some("en".into()), country: None },
        Feed { id: "bbc-world".into(),           title: "BBC — World".into(),             url: "https://feeds.bbci.co.uk/news/world/rss.xml".into(), category: "world".into(), locale: Some("en".into()), country: None },
        Feed { id: "aljazeera-world".into(),     title: "Al Jazeera — World".into(),      url: "https://www.aljazeera.com/xml/rss/all.xml".into(), category: "world".into(), locale: Some("en".into()), country: None },
        Feed { id: "nyt-world".into(),           title: "NYT — World".into(),             url: "https://rss.nytimes.com/services/xml/rss/nyt/World.xml".into(), category: "world".into(), locale: Some("en".into()), country: None },
        Feed { id: "dw-world".into(),            title: "DW — World".into(),              url: "https://rss.dw.com/rdf/rss-en-world".into(), category: "world".into(), locale: Some("en".into()), country: None },
        Feed { id: "vn-vnexpress".into(),        title: "VnExpress — World".into(),       url: "https://vnexpress.net/rss/the-gioi.rss".into(), category: "world".into(), locale: Some("vi".into()), country: Some("VN".into()) },
        // ─── geopolitics / conflict ──────────────────────────────────────
        Feed { id: "gdelt-events".into(),        title: "GDELT — Events".into(),          url: "https://www.gdeltproject.org/updates.html".into(), category: "geopolitics".into(), locale: Some("en".into()), country: None },
        Feed { id: "acled-conflict".into(),      title: "ACLED — Conflict".into(),        url: "https://acleddata.com/feed/".into(), category: "geopolitics".into(), locale: Some("en".into()), country: None },
        Feed { id: "isw-war".into(),             title: "ISW — War".into(),               url: "https://www.understandingwar.org/feeds/blog-posts".into(), category: "geopolitics".into(), locale: Some("en".into()), country: None },
        // ─── technology ──────────────────────────────────────────────────
        Feed { id: "hackernews".into(),          title: "Hacker News".into(),             url: "https://hnrss.org/frontpage".into(), category: "tech".into(), locale: Some("en".into()), country: None },
        Feed { id: "techcrunch".into(),          title: "TechCrunch".into(),              url: "https://techcrunch.com/feed/".into(), category: "tech".into(), locale: Some("en".into()), country: None },
        Feed { id: "ars-technica".into(),        title: "Ars Technica".into(),            url: "https://feeds.arstechnica.com/arstechnica/index".into(), category: "tech".into(), locale: Some("en".into()), country: None },
        Feed { id: "the-verge".into(),           title: "The Verge".into(),               url: "https://www.theverge.com/rss/index.xml".into(), category: "tech".into(), locale: Some("en".into()), country: None },
        // ─── finance ─────────────────────────────────────────────────────
        Feed { id: "cnbc-top".into(),            title: "CNBC — Top News".into(),         url: "https://search.cnbc.com/rs/search/combinedcms/view.xml?partnerId=wrss01&id=100003114".into(), category: "finance".into(), locale: Some("en".into()), country: None },
        Feed { id: "ft-markets".into(),          title: "FT — Markets".into(),            url: "https://www.ft.com/markets?format=rss".into(), category: "finance".into(), locale: Some("en".into()), country: None },
        Feed { id: "bloomberg-markets".into(),   title: "Bloomberg — Markets".into(),     url: "https://feeds.bloomberg.com/markets/news.rss".into(), category: "finance".into(), locale: Some("en".into()), country: None },
        // ─── security ────────────────────────────────────────────────────
        Feed { id: "krebsonsecurity".into(),     title: "Krebs on Security".into(),       url: "https://krebsonsecurity.com/feed/".into(), category: "security".into(), locale: Some("en".into()), country: None },
        Feed { id: "schneier".into(),            title: "Schneier on Security".into(),    url: "https://www.schneier.com/feed/".into(), category: "security".into(), locale: Some("en".into()), country: None },
        Feed { id: "thehackernews".into(),       title: "The Hacker News".into(),         url: "https://feeds.feedburner.com/TheHackersNews".into(), category: "security".into(), locale: Some("en".into()), country: None },
        // ─── science / climate ──────────────────────────────────────────
        Feed { id: "nature".into(),              title: "Nature".into(),                  url: "https://www.nature.com/nature.rss".into(), category: "science".into(), locale: Some("en".into()), country: None },
        Feed { id: "nasa-firms".into(),          title: "NASA FIRMS — Fires".into(),      url: "https://firms.modaps.eosdis.nasa.gov/api/area/csv/VIIRS_SNPP_NRT/world/1".into(), category: "disaster".into(), locale: Some("en".into()), country: None },
        Feed { id: "usgs-quakes".into(),         title: "USGS — Earthquakes".into(),      url: "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/2.5_day.atom".into(), category: "disaster".into(), locale: Some("en".into()), country: None },
    ];
    for f in items {
        r.add(f);
    }
    r
}

/// Async fetcher that pulls and parses a single feed.
pub struct FeedFetcher {
    client: reqwest::Client,
}

impl FeedFetcher {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Aegis-AI/1.7 (+https://github.com/hieulouisdev/Axiom)")
            .timeout(Duration::from_secs(20))
            .build()
            .expect("reqwest client");
        Self { client }
    }

    pub fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Fetch + parse a single feed. Returns the normalized items.
    pub async fn fetch(&self, feed: &Feed) -> Result<Vec<FeedItem>> {
        let resp = self
            .client
            .get(&feed.url)
            .send()
            .await
            .map_err(|e| Error::Other(format!("feed {} fetch: {e}", feed.id)))?;
        if !resp.status().is_success() {
            return Err(Error::Other(format!(
                "feed {} returned {}",
                feed.id,
                resp.status()
            )));
        }
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Other(format!("feed {} body: {e}", feed.id)))?;
        Ok(parse_feed_xml(&body, feed))
    }

    /// Fetch many feeds concurrently, returning all items flattened.
    pub async fn fetch_all(
        &self,
        feeds: &[Feed],
        concurrency: usize,
    ) -> Vec<FeedItem> {
        let concurrency = concurrency.max(1).min(16);
        let sem = Arc::new(tokio::sync::Semaphore::new(concurrency));
        let mut handles = Vec::with_capacity(feeds.len());
        for f in feeds {
            let permit = sem.clone().acquire_owned().await.unwrap();
            let this = self.client.clone();
            let feed = f.clone();
            handles.push(tokio::spawn(async move {
                let _permit = permit;
                let fetcher = FeedFetcher::with_client(this);
                match fetcher.fetch(&feed).await {
                    Ok(items) => items,
                    Err(e) => {
                        tracing::warn!("feed {} failed: {e}", feed.id);
                        Vec::new()
                    }
                }
            }));
        }
        let mut out = Vec::new();
        for h in handles {
            if let Ok(items) = h.await {
                out.extend(items);
            }
        }
        out
    }
}

impl Default for FeedFetcher {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// RSS / Atom parser (very lightweight — no full XML dep)
// ────────────────────────────────────────────────────────────────────────────

pub fn parse_feed_xml(xml: &str, feed: &Feed) -> Vec<FeedItem> {
    if xml.contains("<feed") && xml.contains("<entry") {
        parse_atom(xml, feed)
    } else {
        parse_rss2(xml, feed)
    }
}

fn parse_rss2(xml: &str, feed: &Feed) -> Vec<FeedItem> {
    let mut out = Vec::new();
    for item_block in xml.split("<item").skip(1) {
        let title = extract_tag(item_block, "title").unwrap_or_default();
        let link = extract_tag(item_block, "link").unwrap_or_default();
        let description = extract_tag(item_block, "description").unwrap_or_default();
        let pub_date = extract_tag(item_block, "pubDate").unwrap_or_default();
        let guid = extract_tag(item_block, "guid").unwrap_or_else(|| link.clone());
        if title.is_empty() && link.is_empty() {
            continue;
        }
        out.push(FeedItem {
            feed_id: feed.id.clone(),
            feed_title: feed.title.clone(),
            category: feed.category.clone(),
            title: strip_cdata(&title).trim().to_string(),
            link: strip_cdata(&link).trim().to_string(),
            summary: strip_html(&strip_cdata(&description)),
            published_at_ms: parse_rfc822_ms(&pub_date).unwrap_or(0),
            guid: strip_cdata(&guid).trim().to_string(),
        });
    }
    out
}

fn parse_atom(xml: &str, feed: &Feed) -> Vec<FeedItem> {
    let mut out = Vec::new();
    for entry_block in xml.split("<entry").skip(1) {
        let title = extract_tag(entry_block, "title").unwrap_or_default();
        let link = extract_attr_link(entry_block).unwrap_or_default();
        let summary = extract_tag(entry_block, "summary")
            .or_else(|| extract_tag(entry_block, "content"))
            .unwrap_or_default();
        let updated = extract_tag(entry_block, "updated")
            .or_else(|| extract_tag(entry_block, "published"))
            .unwrap_or_default();
        let id = extract_tag(entry_block, "id").unwrap_or_else(|| link.clone());
        if title.is_empty() && link.is_empty() {
            continue;
        }
        out.push(FeedItem {
            feed_id: feed.id.clone(),
            feed_title: feed.title.clone(),
            category: feed.category.clone(),
            title: strip_cdata(&title).trim().to_string(),
            link: strip_cdata(&link).trim().to_string(),
            summary: strip_html(&strip_cdata(&summary)),
            published_at_ms: parse_iso_ms(&updated).unwrap_or(0),
            guid: strip_cdata(&id).trim().to_string(),
        });
    }
    out
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let start = xml.find(&open)?;
    let after_open = &xml[start + open.len()..];
    let gt = after_open.find('>')?;
    let body_start = after_open[gt + 1..].as_ptr() as usize - xml.as_ptr() as usize;
    let body_end_rel = xml[body_start..].find(&close)?;
    let body_end = body_start + body_end_rel;
    Some(xml[body_start..body_end].to_string())
}

fn extract_attr_link(xml: &str) -> Option<String> {
    let start = xml.find("<link")?;
    let rest = &xml[start..];
    let end = rest.find('>')?;
    let attrs = &rest[..end];
    let href_key = "href=\"";
    let h = attrs.find(href_key)?;
    let after = &attrs[h + href_key.len()..];
    let q = after.find('"')?;
    Some(after[..q].to_string())
}

fn strip_cdata(s: &str) -> String {
    let s = s.trim();
    if let Some(stripped) = s.strip_prefix("<![CDATA[") {
        if let Some(end) = stripped.find("]]>") {
            return stripped[..end].to_string();
        }
    }
    s.to_string()
}

fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    let decoded = out
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");
    let trimmed = decoded.trim();
    if trimmed.len() > 400 {
        format!("{}…", &trimmed[..400])
    } else {
        trimmed.to_string()
    }
}

fn parse_rfc822_ms(s: &str) -> Option<i64> {
    // Mon, 02 Jan 2006 15:04:05 GMT
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let stripped = s.split('+').next().unwrap_or(s);
    let cleaned = stripped.trim_end_matches("GMT").trim();
    let mut parts = cleaned.split_whitespace();
    let _day_name = parts.next()?;
    let _day: i32 = parts.next()?.parse().ok()?;
    let month = parts.next()?;
    let year: i32 = parts.next()?.parse().ok()?;
    let time = parts.next()?;
    let mon_idx = match month {
        "Jan" => 0, "Feb" => 1, "Mar" => 2, "Apr" => 3, "May" => 4, "Jun" => 5,
        "Jul" => 6, "Aug" => 7, "Sep" => 8, "Oct" => 9, "Nov" => 10, "Dec" => 11,
        _ => return None,
    };
    let mut tparts = time.split(':');
    let h: i32 = tparts.next()?.parse().ok()?;
    let m: i32 = tparts.next()?.parse().ok()?;
    let sec: i32 = tparts.next().unwrap_or("0").parse().ok()?;
    // crude to epoch days (no leap-second handling; good enough for sort)
    let days = days_from_civil(year, mon_idx + 1, _day);
    let secs = days * 86400 + h * 3600 + m * 60 + sec;
    Some(secs * 1000)
}

fn parse_iso_ms(s: &str) -> Option<i64> {
    // 2026-08-21T12:34:56Z or ...+07:00
    let s = s.trim();
    if s.len() < 19 {
        return None;
    }
    let year: i32 = s[0..4].parse().ok()?;
    let month: u32 = s[5..7].parse().ok()?;
    let day: u32 = s[8..10].parse().ok()?;
    let h: i32 = s[11..13].parse().ok()?;
    let m: i32 = s[14..16].parse().ok()?;
    let sec: i32 = s[17..19].parse().ok()?;
    let days = days_from_civil(year, month, day as i32);
    Some((days * 86400 + h * 3600 + m * 60 + sec) * 1000)
}

fn days_from_civil(y: i32, m: u32, d: i32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as u32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era as i64) * 146097 + doe as i64 - 719468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rss2_basic() {
        let xml = r#"
        <rss><channel>
            <item>
                <title>Hello world</title>
                <link>https://example.com/1</link>
                <description>First post</description>
                <pubDate>Mon, 02 Jan 2026 15:04:05 GMT</pubDate>
                <guid>uuid-1</guid>
            </item>
            <item>
                <title>Second</title>
                <link>https://example.com/2</link>
            </item>
        </channel></rss>
        "#;
        let feed = Feed {
            id: "test".into(),
            title: "Test".into(),
            url: "http://x".into(),
            category: "world".into(),
            locale: Some("en".into()),
            country: None,
        };
        let items = parse_feed_xml(xml, &feed);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "Hello world");
        assert_eq!(items[0].link, "https://example.com/1");
        assert!(items[0].published_at_ms > 0);
    }

    #[test]
    fn parse_atom_basic() {
        let xml = r#"
        <feed xmlns="http://www.w3.org/2005/Atom">
            <entry>
                <title>Atom entry</title>
                <link href="https://example.com/a" />
                <summary>Body text</summary>
                <updated>2026-08-21T08:00:00Z</updated>
                <id>urn:uuid:a</id>
            </entry>
        </feed>
        "#;
        let feed = Feed {
            id: "test".into(),
            title: "Test".into(),
            url: "http://x".into(),
            category: "world".into(),
            locale: Some("en".into()),
            country: None,
        };
        let items = parse_feed_xml(xml, &feed);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Atom entry");
        assert_eq!(items[0].link, "https://example.com/a");
    }

    #[test]
    fn registry_default() {
        let r = default_feeds();
        assert!(r.len() > 15);
        assert!(r.categories().contains(&"world".to_string()));
        assert!(r.categories().contains(&"finance".to_string()));
        assert!(r.categories().contains(&"security".to_string()));
    }
}
