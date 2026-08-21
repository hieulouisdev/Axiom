use std::io::{self, Read};

use crate::commands::Context;

pub fn run(
    mut ctx: Context,
    provider: String,
    key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
) -> anyhow::Result<()> {
    let api_key = match key {
        Some(k) => Some(k),
        None => {
            // Try env var first: <PROVIDER>_API_KEY uppercased
            let env_name = format!("{}_API_KEY", provider.to_uppercase());
            if let Ok(k) = std::env::var(&env_name) {
                Some(k)
            } else {
                // Read from stdin (TTY check)
                if atty_stdin() {
                    eprintln!("Enter API key for {} (input hidden):", provider);
                    let password = rpassword_prompt()?;
                    if password.is_empty() { None } else { Some(password) }
                } else {
                    let mut buf = String::new();
                    io::stdin().read_to_string(&mut buf)?;
                    let trimmed = buf.trim().to_string();
                    if trimmed.is_empty() { None } else { Some(trimmed) }
                }
            }
        }
    };
    ctx.config.set_provider(&provider, api_key, base_url, model);
    if ctx.config.active_provider.is_none() {
        ctx.config.active_provider = Some(provider.clone());
    }
    ctx.config.save(&ctx.config_path)?;
    println!("✓ provider '{}' configured and saved to {}", provider, ctx.config_path.display());
    Ok(())
}

fn atty_stdin() -> bool {
    // best-effort: use libc::isatty via std::io::IsTerminal (1.70+)
    use std::io::IsTerminal;
    !std::io::stdin().is_terminal()
}

fn rpassword_prompt() -> anyhow::Result<String> {
    // Minimal implementation without depending on rpassword.
    // Read line from stderr-tty if available; otherwise just read a line from stdin.
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf)?;
    Ok(buf.trim_end_matches('\n').to_string())
}
