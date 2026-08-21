use crate::commands::Context;
use crate::CodeAction;

pub fn run(ctx: Context, action: CodeAction, json_mode: bool) -> anyhow::Result<()> {
    match action {
        CodeAction::Register { path, language, name } => {
            let abs = std::fs::canonicalize(&path)?;
            let name = name.unwrap_or_else(|| {
                abs.file_name().and_then(|n| n.to_str()).unwrap_or("repo").to_string()
            });
            let id = ctx.memory.code_graph.register_repo(abs.to_str().unwrap(), &name, &language)?;
            println!("✓ repo #{} '{}' registered ({})", id, name, abs.display());
            println!("  index with: aegis code index {}", id);
        }
        CodeAction::Repos => {
            let r = ctx.memory.code_graph.list_repos()?;
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&r)?);
            } else if r.is_empty() {
                println!("(no repos registered — `aegis code register <path>` to add one)");
            } else {
                println!("Repos ({}):", r.len());
                for repo in r {
                    println!("  #{} {} [{}] {} symbols, {} edges — {}",
                        repo.id, repo.name, repo.language, repo.symbol_count, repo.edge_count,
                        repo.root_path);
                }
            }
        }
        CodeAction::Index { repo_id } => {
            let repos = ctx.memory.code_graph.list_repos()?;
            let repo = repos.iter().find(|r| r.id == repo_id)
                .ok_or_else(|| anyhow::anyhow!("repo #{} not found", repo_id))?;
            let root = std::path::PathBuf::from(&repo.root_path);
            let (s, e) = ctx.memory.code_graph.index_dir(repo_id, &root)?;
            println!("✓ indexed {} — {} symbols, {} edges", repo.name, s, e);
        }
        CodeAction::Search { name } => {
            let r = ctx.memory.code_graph.search_symbols(&name, 50)?;
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&r)?);
            } else if r.is_empty() {
                println!("(no symbols match '{}')", name);
            } else {
                println!("Symbols matching '{}' ({}):", name, r.len());
                for s in r {
                    println!("  #{} [{:>10}] {} — {}:{}",
                        s.id, s.kind.as_str(), s.name, s.file_path, s.line);
                }
            }
        }
        CodeAction::Callers { symbol_id } => {
            let r = ctx.memory.code_graph.callers_of(symbol_id)?;
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&r)?);
            } else {
                println!("Callers of #{} ({}):", symbol_id, r.len());
                for s in r {
                    println!("  #{:<5} [{:>10}] {} — {}:{}", s.id, s.kind.as_str(), s.name, s.file_path, s.line);
                }
            }
        }
        CodeAction::Callees { symbol_id } => {
            let r = ctx.memory.code_graph.callees_of(symbol_id)?;
            if json_mode {
                println!("{}", serde_json::to_string_pretty(&r)?);
            } else {
                println!("Callees of #{} ({}):", symbol_id, r.len());
                for s in r {
                    println!("  #{:<5} [{:>10}] {} — {}:{}", s.id, s.kind.as_str(), s.name, s.file_path, s.line);
                }
            }
        }
    }
    Ok(())
}
