use crate::commands::Context;

pub fn run(ctx: Context, json_mode: bool) -> anyhow::Result<()> {
    let reg = ctx.config.provider_registry();
    let active = ctx.config.active_provider.as_deref().unwrap_or("(none)");
    if json_mode {
        let mut arr = Vec::new();
        for c in reg.list() {
            arr.push(serde_json::json!({
                "id": c.id,
                "has_key": c.api_key.is_some(),
                "base_url": c.base_url,
                "default_model": c.default_model,
                "active": c.id == active,
            }));
        }
        println!("{}", serde_json::to_string_pretty(&serde_json::Value::Array(arr))?);
    } else {
        println!("Aegis AI providers (active: {})", active);
        println!("{:-<60}", "");
        for c in reg.list() {
            let star = if c.id == active { "*" } else { " " };
            let key_status = if c.api_key.is_some() { "[key]" } else { "[no key]" };
            println!("{} {:<15} {:<7} {:<30} model={}",
                star, c.id, key_status,
                c.base_url.as_deref().unwrap_or("default"),
                c.default_model.as_deref().unwrap_or("(default)"));
        }
        println!("\n* = active    use `aegis configure <id>` to set the active provider");
    }
    Ok(())
}
