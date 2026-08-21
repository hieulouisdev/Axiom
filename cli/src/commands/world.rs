use crate::commands::Context;
use crate::WorldAction;

pub async fn run(ctx: Context, action: WorldAction, json_mode: bool) -> anyhow::Result<()> {
    match action {
        WorldAction::News { category, limit } => {
            let agg = crate::world::NewsAggregator::new();
            let briefs = if let Some(cat) = category {
                agg.fetch_category(&cat, limit).await
            } else {
                agg.fetch_all(limit).await
            };
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&briefs)?);
            } else if briefs.is_empty() {
                println!("(no news fetched — check network or feed URLs)");
            } else {
                println!("World news ({} items):", briefs.len());
                for (i, b) in briefs.iter().enumerate() {
                    println!("\n{}. [{}] {} (salience={:.2})", i + 1, b.category.as_str(), b.title, b.salience);
                    println!("   {} — {}", b.feed_title, b.link);
                    if !b.summary.is_empty() {
                        println!("   {}", b.summary.chars().take(200).collect::<String>());
                    }
                }
            }
        }
        WorldAction::Finance { symbols } => {
            let quotes = if symbols.is_empty() {
                crate::world::fetch_market_snapshot().await.quotes
            } else {
                crate::world::fetch_multi(&symbols).await
            };
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&quotes)?);
            } else {
                println!("Market quotes ({}):", quotes.len());
                println!("{}", crate::world::render_quotes(&quotes));
            }
        }
        WorldAction::Snapshot => {
            let snap = crate::world::fetch_market_snapshot().await;
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&snap)?);
            } else {
                println!("Market snapshot ({} quotes):", snap.quotes.len());
                println!("{}", crate::world::render_quotes(&snap.quotes));
            }
        }
        WorldAction::Risk { countries } => {
            // Parse countries: ISO3:Name pairs
            let items: Vec<(String, String, crate::world::InstabilityIndex)> = countries.iter().map(|c| {
                let parts: Vec<&str> = c.splitn(2, ':').collect();
                let iso3 = parts[0].to_string();
                let name = if parts.len() > 1 { parts[1].to_string() } else { iso3.clone() };
                (iso3, name, crate::world::InstabilityIndex::default())
            }).collect();
            let risks = crate::world::score_countries(&items);
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&risks)?);
            } else {
                println!("{}", crate::world::render_country_risks(&risks));
            }
        }
        WorldAction::Brief { title } => {
            let agg = crate::world::NewsAggregator::new();
            let news = agg.fetch_all(15).await;
            let snap = crate::world::fetch_market_snapshot().await;
            let brief = crate::world::BriefComposer::compose(&title, &news, &snap.quotes, &[]);
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&brief)?);
            } else {
                println!("{}", brief.as_text);
            }
        }
    }
    Ok(())
}
