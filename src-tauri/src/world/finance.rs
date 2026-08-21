//! v1.7.0 — Market data (stocks, FX, crypto, commodities).
//!
//! Uses free, no-API-key-required endpoints:
//!
//! - **Crypto**: CoinGecko v3 simple/price (`/simple/price?ids=bitcoin&vs_currencies=usd`)
//! - **FX**: European Central Bank daily reference rates (XML, free)
//! - **Stocks**: Stooq.com (CSV, free, no key)
//!
//! All providers are async and return normalized [`FinanceQuote`] structs.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinanceQuote {
    pub symbol: String,
    pub price: f64,
    pub currency: String,
    pub change_pct: Option<f64>,
    pub source: String,
    pub fetched_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSnapshot {
    pub label: String,
    pub quotes: Vec<FinanceQuote>,
    pub fetched_at_ms: i64,
}

/// Fetch a single quote. The provider is auto-detected from the symbol:
///
/// - prefixed `crypto:` → CoinGecko (e.g. `crypto:bitcoin`)
/// - prefixed `fx:`     → ECB FX (e.g. `fx:EURUSD`)
/// - otherwise          → Stooq (e.g. `AAPL`, `MSFT`, `BTC` for crypto via stooq)
pub async fn fetch_quote(symbol: &str) -> Result<FinanceQuote> {
    let client = reqwest::Client::builder()
        .user_agent("Aegis-AI/1.7 (+https://github.com/hieulouisdev/Axiom)")
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| Error::Other(format!("http client: {e}")))?;
    fetch_quote_with(&client, symbol).await
}

async fn fetch_quote_with(client: &reqwest::Client, symbol: &str) -> Result<FinanceQuote> {
    if let Some(id) = symbol.strip_prefix("crypto:") {
        fetch_crypto(client, id).await
    } else if let Some(pair) = symbol.strip_prefix("fx:") {
        fetch_fx(client, pair).await
    } else {
        fetch_stock(client, symbol).await
    }
}

/// Fetch many quotes concurrently. Unknown symbols are silently dropped.
pub async fn fetch_multi(symbols: &[String]) -> Vec<FinanceQuote> {
    let client = reqwest::Client::builder()
        .user_agent("Aegis-AI/1.7 (+https://github.com/hieulouisdev/Axiom)")
        .timeout(Duration::from_secs(15))
        .build()
        .expect("reqwest client");
    let mut handles = Vec::with_capacity(symbols.len());
    for s in symbols {
        let c = client.clone();
        let sym = s.clone();
        handles.push(tokio::spawn(async move {
            match fetch_quote_with(&c, &sym).await {
                Ok(q) => Some(q),
                Err(e) => {
                    tracing::warn!("fetch_quote({}) failed: {e}", sym);
                    None
                }
            }
        }));
    }
    let mut out = Vec::with_capacity(symbols.len());
    for h in handles {
        if let Ok(Some(q)) = h.await {
            out.push(q);
        }
    }
    out
}

/// Fetch a snapshot of common market indices + crypto majors + FX majors.
pub async fn fetch_market_snapshot() -> MarketSnapshot {
    let symbols = vec![
        // stocks
        "^spx".into(),    // S&P 500
        "^ndq".into(),    // Nasdaq
        "^dji".into(),    // Dow Jones
        // crypto
        "crypto:bitcoin".into(),
        "crypto:ethereum".into(),
        "crypto:solana".into(),
        // FX
        "fx:EURUSD".into(),
        "fx:GBPUSD".into(),
        "fx:USDJPY".into(),
        "fx:USDVND".into(),
    ];
    let quotes = fetch_multi(&symbols).await;
    let fetched_at_ms = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
    MarketSnapshot {
        label: "Global market snapshot".into(),
        quotes,
        fetched_at_ms,
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Providers
// ────────────────────────────────────────────────────────────────────────────

async fn fetch_crypto(client: &reqwest::Client, coin_id: &str) -> Result<FinanceQuote> {
    let url = format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&include_24hr_change=true"
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| Error::Other(format!("coingecko: {e}")))?;
    if !resp.status().is_success() {
        return Err(Error::Other(format!("coingecko returned {}", resp.status())));
    }
    let body = resp.text().await.map_err(|e| Error::Other(format!("coingecko body: {e}")))?;
    let parsed: HashMap<String, HashMap<String, f64>> = serde_json::from_str(&body)
        .map_err(|e| Error::Other(format!("coingecko json: {e}")))?;
    let entry = parsed
        .get(coin_id)
        .ok_or_else(|| Error::Other(format!("coingecko: coin '{}' not in response", coin_id)))?;
    let price = entry
        .get("usd")
        .copied()
        .ok_or_else(|| Error::Other("coingecko: missing 'usd' field".into()))?;
    let change = entry.get("usd_24h_change").copied();
    Ok(FinanceQuote {
        symbol: format!("crypto:{}", coin_id),
        price,
        currency: "USD".into(),
        change_pct: change,
        source: "CoinGecko".into(),
        fetched_at_ms: time::OffsetDateTime::now_utc().unix_timestamp() * 1000,
    })
}

async fn fetch_fx(client: &reqwest::Client, pair: &str) -> Result<FinanceQuote> {
    // ECB publishes daily reference rates in XML. Pair is e.g. "EURUSD".
    // We fetch EUR-base and invert if the user wants USD-base.
    if pair.len() != 6 {
        return Err(Error::Other(format!("fx pair must be 6 chars, got '{}'", pair)));
    }
    let base = &pair[..3];
    let quote = &pair[3..];
    let url = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml";
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| Error::Other(format!("ecb: {e}")))?;
    if !resp.status().is_success() {
        return Err(Error::Other(format!("ecb returned {}", resp.status())));
    }
    let body = resp.text().await.map_err(|e| Error::Other(format!("ecb body: {e}")))?;
    let rates = parse_ecb_rates(&body);
    let price = if base == "EUR" {
        rates.get(quote).copied().ok_or_else(|| Error::Other(format!("ecb: no rate for {}", quote)))?
    } else if quote == "EUR" {
        let r = rates.get(base).copied().ok_or_else(|| Error::Other(format!("ecb: no rate for {}", base)))?;
        1.0 / r
    } else {
        // cross-rate via EUR
        let base_to_eur = rates.get(base).copied().ok_or_else(|| Error::Other(format!("ecb: no rate for {}", base)))?;
        let eur_to_quote = rates.get(quote).copied().ok_or_else(|| Error::Other(format!("ecb: no rate for {}", quote)))?;
        eur_to_quote / base_to_eur
    };
    Ok(FinanceQuote {
        symbol: format!("fx:{}", pair),
        price,
        currency: quote.into(),
        change_pct: None,
        source: "ECB".into(),
        fetched_at_ms: time::OffsetDateTime::now_utc().unix_timestamp() * 1000,
    })
}

fn parse_ecb_rates(xml: &str) -> HashMap<String, f64> {
    let mut out = HashMap::new();
    // crude: find every <Cube currency='XXX' rate='1.2345'/>
    for chunk in xml.split("<Cube currency=") {
        if chunk.is_empty() {
            continue;
        }
        let q1 = chunk.find('\'').or_else(|| chunk.find('"'));
        let q1 = match q1 {
            Some(i) => i + 1,
            None => continue,
        };
        let rest = &chunk[q1..];
        let q2 = match rest.find('\'').or_else(|| rest.find('"')) {
            Some(i) => i,
            None => continue,
        };
        let ccy = &rest[..q2];
        // find rate=
        if let Some(r_idx) = chunk.find("rate=") {
            let r_rest = &chunk[r_idx + 5..];
            let r_q1 = r_rest.find('\'').or_else(|| r_rest.find('"'));
            if let Some(i) = r_q1 {
                let rr = &r_rest[i + 1..];
                if let Some(end) = rr.find('\'').or_else(|| rr.find('"')) {
                    if let Ok(rate) = rr[..end].parse::<f64>() {
                        out.insert(ccy.to_string(), rate);
                    }
                }
            }
        }
    }
    out
}

async fn fetch_stock(client: &reqwest::Client, symbol: &str) -> Result<FinanceQuote> {
    // Stooq returns CSV: Symbol,Date,Time,Open,High,Low,Close,Volume
    let stooq_sym = symbol.replace('^', "");
    let url = format!("https://stooq.com/q/l/?s={}&f=sd2t2ohlcv&h&e=csv", stooq_sym.to_lowercase());
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| Error::Other(format!("stooq: {e}")))?;
    if !resp.status().is_success() {
        return Err(Error::Other(format!("stooq returned {}", resp.status())));
    }
    let body = resp.text().await.map_err(|e| Error::Other(format!("stooq body: {e}")))?;
    let mut lines = body.lines();
    let _header = lines.next();
    let row = lines.next().ok_or_else(|| Error::Other("stooq: empty csv".into()))?;
    let cols: Vec<&str> = row.split(',').collect();
    if cols.len() < 7 {
        return Err(Error::Other(format!("stooq: malformed row: {row}")));
    }
    let close: f64 = cols[6].parse().map_err(|_| Error::Other(format!("stooq: bad close: {}", cols[6])))?;
    let open: f64 = cols[3].parse().unwrap_or(close);
    let change_pct = if open > 0.0 {
        Some((close - open) / open * 100.0)
    } else {
        None
    };
    Ok(FinanceQuote {
        symbol: symbol.into(),
        price: close,
        currency: "USD".into(),
        change_pct,
        source: "Stooq".into(),
        fetched_at_ms: time::OffsetDateTime::now_utc().unix_timestamp() * 1000,
    })
}

/// Render a list of quotes as a compact text block.
pub fn render_quotes(quotes: &[FinanceQuote]) -> String {
    if quotes.is_empty() {
        return "(no quotes available)".into();
    }
    let mut out = String::with_capacity(512);
    for q in quotes {
        let change_str = match q.change_pct {
            Some(c) => format!(" ({:+.2}%)", c),
            None => String::new(),
        };
        out.push_str(&format!(
            "- {} {:.4} {}{} — via {}\n",
            q.symbol, q.price, q.currency, change_str, q.source
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ecb_smoke() {
        let xml = r#"
        <gesmes:Envelope>
          <Cube>
            <Cube time="2026-08-21">
              <Cube currency="USD" rate="1.0876"/>
              <Cube currency="JPY" rate="161.32"/>
              <Cube currency="VND" rate="27500.0"/>
            </Cube>
          </Cube>
        </gesmes:Envelope>
        "#;
        let r = parse_ecb_rates(xml);
        assert_eq!(r.get("USD"), Some(&1.0876));
        assert_eq!(r.get("JPY"), Some(&161.32));
        assert_eq!(r.get("VND"), Some(&27500.0));
    }

    #[test]
    fn render_quotes_smoke() {
        let qs = vec![FinanceQuote {
            symbol: "AAPL".into(),
            price: 225.0,
            currency: "USD".into(),
            change_pct: Some(1.2),
            source: "Stooq".into(),
            fetched_at_ms: 0,
        }];
        let s = render_quotes(&qs);
        assert!(s.contains("AAPL"));
        assert!(s.contains("+1.20%"));
    }
}
