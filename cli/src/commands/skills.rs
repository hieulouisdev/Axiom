use crate::commands::Context;
use crate::SkillsAction;
use crate::memory::{SkillStatus, Visibility, SkillTrigger};

pub fn run(ctx: Context, action: SkillsAction, json_mode: bool) -> anyhow::Result<()> {
    match action {
        SkillsAction::List => {
            let s = ctx.memory.skills.list_skills(false)?;
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&s)?);
            } else if s.is_empty() {
                println!("(no skills yet — `aegis skills create <slug> <name> <desc>` to create one)");
            } else {
                println!("Skills ({}):", s.len());
                for sk in s {
                    println!("  [{:>9}] {:<25} {:<40} v{}",
                        sk.current_status.as_str(), sk.slug, sk.name, sk.current_version);
                }
            }
        }
        SkillsAction::Show { slug } => {
            let sk = ctx.memory.skills.get_skill_by_slug(&slug)?
                .ok_or_else(|| anyhow::anyhow!("skill '{}' not found", slug))?;
            let v = ctx.memory.skills.load_published_version(sk.id)?;
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({"skill": sk, "version": v}))?);
            } else {
                println!("Skill: {} ({})", sk.name, sk.slug);
                println!("  status: {}, version: {}, visibility: {}", sk.current_status.as_str(), sk.current_version, sk.visibility.as_str());
                if let Some(v) = v {
                    println!("  trigger keywords: {}", v.trigger.keywords.join(", "));
                    println!("  system prompt:\n    {}", v.system_prompt.replace('\n', "\n    "));
                }
            }
        }
        SkillsAction::Match { message } => {
            let matched = ctx.memory.skills.match_triggers(&message)?;
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({"matched": matched}))?);
            } else {
                if matched.is_empty() {
                    println!("(no skill triggers matched)");
                } else {
                    println!("Matched skills: {}", matched.join(", "));
                }
            }
        }
        SkillsAction::Create { slug, name, description, visibility } => {
            let v = Visibility::parse(&visibility).ok_or_else(|| anyhow::anyhow!("invalid visibility: {}", visibility))?;
            let id = ctx.memory.skills.create_skill(&slug, &name, &description, v, "cli")?;
            println!("✓ skill #{} '{}' created (draft status)", id, slug);
        }
        SkillsAction::SaveVersion { slug, system_prompt, keywords } => {
            let sk = ctx.memory.skills.get_skill_by_slug(&slug)?
                .ok_or_else(|| anyhow::anyhow!("skill '{}' not found", slug))?;
            let v = ctx.memory.skills.save_version(
                sk.id,
                &system_prompt,
                &SkillTrigger { keywords, intents: vec![] },
                &[],
                None,
            )?;
            println!("✓ version {} saved (draft) for skill '{}'", v, slug);
            println!("  publish with: aegis skills publish {} {}", slug, v);
        }
        SkillsAction::Publish { slug, version } => {
            let sk = ctx.memory.skills.get_skill_by_slug(&slug)?
                .ok_or_else(|| anyhow::anyhow!("skill '{}' not found", slug))?;
            ctx.memory.skills.publish_version(sk.id, version)?;
            println!("✓ version {} of skill '{}' published", version, slug);
        }
    }
    Ok(())
}

// silence unused warnings for SkillStatus re-exported but not directly used here
#[allow(dead_code)]
fn _skill_status_marker(s: SkillStatus) -> &'static str { s.as_str() }
