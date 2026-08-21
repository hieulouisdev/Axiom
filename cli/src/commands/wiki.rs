use crate::commands::Context;
use crate::WikiAction;

pub fn run(ctx: Context, action: WikiAction, json_mode: bool) -> anyhow::Result<()> {
    match action {
        WikiAction::List => {
            let p = ctx.memory.wiki.list_pages()?;
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&p)?);
            } else if p.is_empty() {
                println!("(no wiki pages yet)");
            } else {
                println!("Wiki pages ({}):", p.len());
                for page in p {
                    println!("  {:<25} {:<50} ({} chars)",
                        page.slug, page.title, page.body.len());
                }
            }
        }
        WikiAction::Search { query } => {
            let r = ctx.memory.wiki.search(&query)?;
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&r)?);
            } else {
                println!("Search '{}' ({} hits):", query, r.len());
                for p in r {
                    println!("  {:<25} {}", p.slug, p.title);
                }
            }
        }
        WikiAction::Show { slug } => {
            let p = ctx.memory.wiki.get_page(&slug)?
                .ok_or_else(|| anyhow::anyhow!("page '{}' not found", slug))?;
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&p)?);
            } else {
                println!("# {}  [{}]", p.title, p.slug);
                if !p.tags.is_empty() {
                    println!("tags: {}", p.tags.join(", "));
                }
                println!("\n{}", p.body);
            }
        }
        WikiAction::Add { slug, title, body, tags } => {
            let body = body.unwrap_or_default();
            ctx.memory.wiki.upsert_page(&slug, &title, &body, &tags, Some("cli"))?;
            println!("✓ wiki page '{}' saved", slug);
        }
        WikiAction::Remove { slug } => {
            ctx.memory.wiki.delete_page(&slug)?;
            println!("✓ wiki page '{}' removed", slug);
        }
    }
    Ok(())
}
