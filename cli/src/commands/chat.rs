use crate::commands::Context;
use crate::ai::provider::{ChatMessage, ChatRequest, Role};
use crate::memory::MessageRole;

pub async fn run(
    ctx: Context,
    message: String,
    provider: Option<String>,
    model: Option<String>,
    json_mode: bool,
) -> anyhow::Result<()> {
    // Save user message to memory (auto-create a conversation per session)
    let conv_id = uuid::Uuid::new_v4().to_string();
    let title: String = message.chars().take(60).collect();
    ctx.memory.conversations.create_conversation(&conv_id, &title, provider.as_deref())?;
    ctx.memory.conversations.append_message(&conv_id, MessageRole::User, &message)?;

    // Build prompt fragment from memory
    let mem_fragment = ctx.memory.hierarchy.render_prompt_fragment(&ctx.config.persona_user_id, 10).unwrap_or_default();

    // Build the request
    let mut messages: Vec<ChatMessage> = Vec::new();
    if !mem_fragment.is_empty() {
        messages.push(ChatMessage {
            role: Role::System,
            content: format!("You are Aegis AI, a secure cross-platform assistant. User context:\n\n{}", mem_fragment),
        });
    } else {
        messages.push(ChatMessage {
            role: Role::System,
            content: "You are Aegis AI, a secure cross-platform assistant.".into(),
        });
    }
    let history = ctx.memory.conversations.list_messages(&conv_id, 20)?;
    for m in history {
        let role = match m.role {
            MessageRole::User => Role::User,
            MessageRole::Assistant => Role::Assistant,
            MessageRole::System => Role::System,
            MessageRole::Tool => Role::User,
        };
        messages.push(ChatMessage { role, content: m.content });
    }

    let req = ChatRequest {
        model: model.unwrap_or_default(),
        messages,
        temperature: Some(0.7),
        max_tokens: Some(2048),
    };

    let provider_id = provider.as_deref().or(ctx.config.active_provider.as_deref()).unwrap_or("zai");
    let resp = ctx.router.chat(&req, Some(provider_id)).await?;

    // Save assistant reply
    ctx.memory.conversations.append_message(&conv_id, MessageRole::Assistant, &resp.content)?;

    // Auto-extract atoms
    let atoms = crate::memory::deterministic_extract(&message);
    let atoms_count = atoms.len();
    for (kind, summary) in atoms {
        let _ = ctx.memory.hierarchy.add_atom(kind, &summary, None, Some(&conv_id), None, 0.7);
    }

    if json_mode {
        let v = serde_json::json!({
            "content": resp.content,
            "model": resp.model,
            "provider": resp.provider,
            "usage": resp.usage,
            "conversation_id": conv_id,
            "atoms_extracted": atoms_count,
        });
        println!("{}", serde_json::to_string_pretty(&v)?);
    } else {
        println!("{}", resp.content);
        if let Some(u) = &resp.usage {
            eprintln!("\n---\nprovider: {} | model: {} | tokens: {} (in) + {} (out) = {}", resp.provider, resp.model, u.prompt_tokens, u.completion_tokens, u.total_tokens);
        }
    }
    Ok(())
}
