//! Concrete provider implementations.
//!
//! Each provider is a thin wrapper around the OpenAI-compatible
//! `/v1/chat/completions` endpoint (which is what most providers expose
//! today). Anthropic and Gemini have their own shapes and get hand-rolled.

pub mod openai;
pub mod openai_compat;
pub mod anthropic;
pub mod gemini;
pub mod deepseek;
pub mod zai;
pub mod ollama;
pub mod openrouter;

/// Helper used by all OpenAI-compatible providers.
pub(crate) fn build_chat_payload(
    req: &crate::ai::ChatRequest,
    model: &str,
) -> serde_json::Value {
    let messages: Vec<serde_json::Value> = req
        .messages
        .iter()
        .map(|m| {
            serde_json::json!({
                "role": m.role.as_str(),
                "content": m.content,
            })
        })
        .collect();
    let mut payload = serde_json::json!({
        "model": model,
        "messages": messages,
    });
    if let Some(t) = req.temperature {
        payload["temperature"] = serde_json::json!(t);
    }
    if let Some(m) = req.max_tokens {
        payload["max_tokens"] = serde_json::json!(m);
    }
    payload
}

pub(crate) fn parse_chat_response(
    body: &str,
    provider: &str,
    default_model: &str,
) -> anyhow::Result<crate::ai::ChatResponse> {
    let v: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| anyhow::anyhow!("invalid JSON from {provider}: {e} — body: {body}"))?;
    let content = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("{provider}: missing choices[0].message.content"))?
        .to_string();
    let model = v["model"].as_str().unwrap_or(default_model).to_string();
    let usage = if v.get("usage").is_some() {
        Some(crate::ai::Usage {
            prompt_tokens: v["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: v["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: v["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
        })
    } else {
        None
    };
    Ok(crate::ai::ChatResponse {
        content,
        model,
        provider: provider.into(),
        usage,
    })
}
