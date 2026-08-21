use crate::commands::Context;
use crate::MemoryAction;
use crate::memory::AtomKind;

pub fn run(ctx: Context, action: MemoryAction, json_mode: bool) -> anyhow::Result<()> {
    match action {
        MemoryAction::Atoms { limit } => {
            let atoms = ctx.memory.hierarchy.list_atoms(limit as i64)?;
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&atoms)?);
            } else if atoms.is_empty() {
                println!("(no memory atoms yet — chat or use `aegis memory add` to add some)");
            } else {
                println!("Memory atoms ({} most recent):", atoms.len());
                println!("{:-<70}", "");
                for a in atoms {
                    println!("[{}] #{} ({}) conf={:.2}  {}",
                        format_ts(a.created_at_ms), a.id, a.kind.as_str(), a.confidence, a.summary);
                }
            }
        }
        MemoryAction::Add { kind, scenario, summary } => {
            let k = AtomKind::parse(&kind).ok_or_else(|| anyhow::anyhow!("invalid kind: {} (allowed: preference, fact, decision, instruction, goal, context)", kind))?;
            let scenario_id = if let Some(title) = scenario {
                Some(ctx.memory.hierarchy.upsert_scenario(&title, None, &[])?)
            } else { None };
            let id = ctx.memory.hierarchy.add_atom(k, &summary, None, None, scenario_id, 0.8)?;
            println!("✓ atom #{} added (kind={}, scenario={:?})", id, k.as_str(), scenario_id);
        }
        MemoryAction::Scenarios => {
            let s = ctx.memory.hierarchy.list_scenarios()?;
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&s)?);
            } else if s.is_empty() {
                println!("(no scenarios yet)");
            } else {
                println!("Scenarios ({}):", s.len());
                for sc in s {
                    let tags = if sc.tags.is_empty() { String::new() } else { format!(" [{}]", sc.tags.join(", ")) };
                    println!("  #{} {} ({} atoms){} — updated {}",
                        sc.id, sc.title, sc.atom_count, tags, format_ts(sc.updated_at_ms));
                }
            }
        }
        MemoryAction::NewScenario { title, summary, tags } => {
            let id = ctx.memory.hierarchy.upsert_scenario(&title, summary.as_deref(), &tags)?;
            println!("✓ scenario #{} '{}' created", id, title);
        }
        MemoryAction::Persona => {
            let p = ctx.memory.hierarchy.load_persona(&ctx.config.persona_user_id)?;
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&p)?);
            } else if p.traits.is_empty() {
                println!("(persona is empty — use `aegis memory set-trait <key> <value>` to add traits)");
            } else {
                println!("Persona ({} traits):", p.traits.len());
                for t in p.traits {
                    println!("  {}: {} (conf={:.2})", t.key, t.value, t.confidence);
                }
            }
        }
        MemoryAction::SetTrait { key, value, confidence } => {
            ctx.memory.hierarchy.set_persona_trait(&ctx.config.persona_user_id, &key, &value, confidence, Some("cli"))?;
            println!("✓ persona trait '{}' set", key);
        }
        MemoryAction::Forget { id } => {
            ctx.memory.hierarchy.forget_atom(id)?;
            println!("✓ atom #{} forgotten", id);
        }
        MemoryAction::Prompt { recent } => {
            let p = ctx.memory.hierarchy.render_prompt_fragment(&ctx.config.persona_user_id, recent)?;
            if p.is_empty() {
                println!("(memory prompt fragment is empty)");
            } else {
                println!("{}", p);
            }
        }
    }
    Ok(())
}

fn format_ts(ms: i64) -> String {
    if ms <= 0 { return "—".into(); }
    let dt = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms).unwrap_or_else(|| chrono::Utc::now());
    dt.format("%Y-%m-%d %H:%M").to_string()
}
