//! Aegis AI CLI — entry point.
//!
//! Two modes:
//! - **Interactive TUI** (default): a beautiful terminal interface with
//!   chat, memory, world intelligence, and skills panels.
//! - **One-shot**: `aegis chat "explain async rust"` — single query,
//!   print answer, exit. Suitable for scripting.

use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod ai;
mod commands;
mod config;
mod mcp;
mod memory;
mod tui;
mod world;

#[derive(Parser, Debug)]
#[command(
    name = "aegis",
    version,
    about = "Aegis AI CLI — secure cross-platform AI assistant",
    long_about = "Aegis AI v1.7.0 — Singularity II upgrade.\n\
                  Features: 90+ AI providers, hierarchical memory, world intelligence,\n\
                  MCP server, beautiful TUI, and one-shot scripting mode."
)]
struct Cli {
    /// Path to the config file (default: platform data dir).
    #[arg(long, env = "AEGIS_CONFIG")]
    config: Option<String>,

    /// Path to the SQLite database (default: platform data dir).
    #[arg(long, env = "AEGIS_DB")]
    db: Option<String>,

    /// JSON output for one-shot commands (no ANSI colors).
    #[arg(long, env = "AEGIS_JSON")]
    json: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Launch the interactive TUI (default when no subcommand is given).
    Tui,
    /// One-shot chat: ask a single question, print answer, exit.
    Chat {
        /// The user message.
        message: String,
        /// Provider override (default: from config or 'zai').
        #[arg(long)]
        provider: Option<String>,
        /// Model override.
        #[arg(long)]
        model: Option<String>,
    },
    /// List configured AI providers.
    Providers,
    /// Configure an AI provider (set API key, base URL).
    Configure {
        /// Provider id, e.g. "openai", "anthropic", "zai", "ollama".
        provider: String,
        /// API key (omit to read from stdin).
        #[arg(long)]
        key: Option<String>,
        /// Override the base URL.
        #[arg(long)]
        base_url: Option<String>,
        /// Override the default model.
        #[arg(long)]
        model: Option<String>,
    },
    /// Memory management (atoms, scenarios, persona, knowledge graph).
    Memory {
        #[command(subcommand)]
        action: MemoryAction,
    },
    /// Skill library (list, show, match triggers).
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },
    /// Wiki knowledge base (list, search, show, add).
    Wiki {
        #[command(subcommand)]
        action: WikiAction,
    },
    /// World intelligence (news, finance, risk).
    World {
        #[command(subcommand)]
        action: WorldAction,
    },
    /// CodeGraph (index, search, callers, callees).
    Code {
        #[command(subcommand)]
        action: CodeAction,
    },
    /// Run the MCP server (JSON-RPC over stdio).
    Mcp,
    /// Show Aegis AI version and build info.
    Version,
}

#[derive(Subcommand, Debug)]
enum MemoryAction {
    /// List recent memory atoms (L1).
    Atoms {
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Add a memory atom manually.
    Add {
        /// Kind: preference, fact, decision, instruction, goal, context.
        #[arg(long, default_value = "fact")]
        kind: String,
        #[arg(long)]
        scenario: Option<String>,
        summary: String,
    },
    /// List all scenarios (L2).
    Scenarios,
    /// Create a new scenario.
    NewScenario {
        title: String,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
    },
    /// Show the user persona (L3).
    Persona,
    /// Set a persona trait.
    SetTrait {
        key: String,
        value: String,
        #[arg(long, default_value_t = 0.8)]
        confidence: f64,
    },
    /// Forget a single atom by id.
    Forget { id: i64 },
    /// Render the memory prompt fragment (what the agent sees).
    Prompt {
        #[arg(long, default_value_t = 10)]
        recent: i64,
    },
}

#[derive(Subcommand, Debug)]
enum SkillsAction {
    /// List all skills.
    List,
    /// Show a skill's current published version.
    Show { slug: String },
    /// Match triggers against a message; print matched slugs.
    Match { message: String },
    /// Create a new skill (draft).
    Create {
        slug: String,
        name: String,
        description: String,
        #[arg(long, default_value = "private")]
        visibility: String,
    },
    /// Save a new version of a skill's content (from stdin or args).
    SaveVersion {
        slug: String,
        #[arg(long)]
        system_prompt: String,
        #[arg(long, value_delimiter = ',')]
        keywords: Vec<String>,
    },
    /// Publish a draft version.
    Publish { slug: String, version: i64 },
}

#[derive(Subcommand, Debug)]
enum WikiAction {
    List,
    Search { query: String },
    Show { slug: String },
    Add {
        slug: String,
        title: String,
        #[arg(long)]
        body: Option<String>,
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,
    },
    Remove { slug: String },
}

#[derive(Subcommand, Debug)]
enum WorldAction {
    /// Fetch latest news briefs.
    News {
        #[arg(long)]
        category: Option<String>,
        #[arg(long, default_value_t = 15)]
        limit: usize,
    },
    /// Fetch market quotes.
    Finance {
        /// Symbols: crypto:bitcoin, fx:EURUSD, AAPL, ^spx, etc.
        symbols: Vec<String>,
    },
    /// Fetch a full market snapshot.
    Snapshot,
    /// Compute country instability index.
    Risk {
        #[arg(long, value_delimiter = ',')]
        countries: Vec<String>,
    },
    /// Compose a daily brief (news + markets + risks).
    Brief {
        #[arg(long, default_value = "Daily Brief")]
        title: String,
    },
}

#[derive(Subcommand, Debug)]
enum CodeAction {
    /// Register a repo for indexing.
    Register {
        /// Absolute path to the repo root.
        path: String,
        #[arg(long, default_value = "rust")]
        language: String,
        #[arg(long)]
        name: Option<String>,
    },
    /// List registered repos.
    Repos,
    /// Index a registered repo.
    Index { repo_id: i64 },
    /// Search indexed symbols.
    Search { name: String },
    /// List callers of a symbol.
    Callers { symbol_id: i64 },
    /// List callees of a symbol.
    Callees { symbol_id: i64 },
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    // Install the rustls crypto provider (ring) at process startup. Required
    // because we use the `rustls-no-provider` feature in reqwest.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let cli = Cli::parse();
    // MCP server uses blocking stdio I/O + nested runtimes; run it outside the
    // tokio async context by spawning a blocking task.
    if let Some(Command::Mcp) = &cli.command {
        let result = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
            let ctx = commands::Context::new(cli.config.as_deref(), cli.db.as_deref())?;
            commands::mcp::run(ctx)
        })
        .await;
        match result {
            Ok(Ok(())) => return ExitCode::SUCCESS,
            Ok(Err(e)) => {
                eprintln!("error: {e:#}");
                return ExitCode::FAILURE;
            }
            Err(e) => {
                eprintln!("join error: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    let ctx = commands::Context::new(cli.config.as_deref(), cli.db.as_deref())?;
    let json_mode = cli.json;

    match cli.command {
        None | Some(Command::Tui) => {
            tui::run(ctx).await?;
        }
        Some(Command::Chat { message, provider, model }) => {
            commands::chat::run(ctx, message, provider, model, json_mode).await?;
        }
        Some(Command::Providers) => {
            commands::providers::run(ctx, json_mode)?;
        }
        Some(Command::Configure { provider, key, base_url, model }) => {
            commands::configure::run(ctx, provider, key, base_url, model)?;
        }
        Some(Command::Memory { action }) => {
            commands::memory::run(ctx, action, json_mode)?;
        }
        Some(Command::Skills { action }) => {
            commands::skills::run(ctx, action, json_mode)?;
        }
        Some(Command::Wiki { action }) => {
            commands::wiki::run(ctx, action, json_mode)?;
        }
        Some(Command::World { action }) => {
            commands::world::run(ctx, action, json_mode).await?;
        }
        Some(Command::Code { action }) => {
            commands::code::run(ctx, action, json_mode)?;
        }
        Some(Command::Mcp) => {
            // unreachable — handled in main() before entering tokio context
        }
        Some(Command::Version) => {
            println!("Aegis AI CLI v1.7.0");
            println!("  Rust toolchain : 1.97.1");
            println!("  Build target   : {}", std::env::consts::ARCH);
            println!("  OS             : {}", std::env::consts::OS);
            println!("  Features       : hierarchical memory, world intelligence, MCP, TUI");
            println!("  Repo           : https://github.com/hieulouisdev/Axiom");
        }
    }
    Ok(())
}
