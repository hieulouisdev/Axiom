use crate::commands::Context;

pub fn run(_ctx: Context) -> anyhow::Result<()> {
    let server = crate::mcp::McpServer::new(
        crate::mcp::McpServerConfig::default(),
        crate::mcp::registry_default(),
    );
    eprintln!("Aegis AI MCP server v1.7.0 starting on stdio...");
    server.run_blocking()?;
    Ok(())
}
