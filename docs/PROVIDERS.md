# Adding a New AI Provider

Guide for adding a new AI provider to Aegis AI.

---

## OpenAI-Compatible Shortcut

If your provider exposes `/v1/chat/completions`, you can add it in **one file, ~15 lines**.

### Step 1: Create the provider file

`src-tauri/src/ai/providers/<your_provider>.rs`:

```rust
use std::sync::Arc;
use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct YourProvider;
impl YourProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "your_provider",                                  // unique id
            "Your Provider Display Name",                     // display name
            "Short description.",                             // description
            "https://your-provider.com",                      // homepage
            ProviderCategory::CloudMajor,                     // category
            true,                                             // requires_api_key
            false,                                            // local?
            Some("https://api.your-provider.com/v1"),         // default base URL
            "your-default-model",                             // default model
            &["model-a", "model-b"],                          // known models
            true,                                             // implemented
        ))
    }
}
```

### Step 2: Register it

In `src-tauri/src/ai/providers/mod.rs`:
```rust
pub mod your_provider;
pub use your_provider::YourProvider;
```

In `src-tauri/src/ai/provider.rs`, add to `ProviderRegistry::with_builtin`:
```rust
register(&mut map, YourProvider::new());
```

The provider appears in the **AI Providers** panel automatically.

---

## Bespoke Client

Write a bespoke client when the API shape differs from OpenAI's (e.g., Anthropic, Gemini, Ollama, Bedrock).

Implement the `Provider` trait directly:

```rust
use async_trait::async_trait;
use crate::ai::provider::{ChatRequest, ChatResponse, Provider, ProviderDescriptor, ProviderCreds};

pub struct YourProvider {
    descriptor: ProviderDescriptor,
    creds: std::sync::RwLock<ProviderCreds>,
    client: reqwest::Client,
}

#[async_trait]
impl Provider for YourProvider {
    fn descriptor(&self) -> &ProviderDescriptor { &self.descriptor }
    fn set_creds(&self, creds: ProviderCreds) { *self.creds.write().unwrap() = creds; }
    async fn chat(&self, req: ChatRequest) -> crate::error::Result<ChatResponse> { /* ... */ }
    async fn ping(&self) -> crate::error::Result<()> { /* ... */ }
}
```

---

## Provider Categories

| Category | Use for |
|---|---|
| `CloudMajor` | Big clouds (OpenAI, Anthropic, Gemini, DeepSeek, Groq, Mistral) |
| `CloudOther` | Smaller/regional clouds (Azure, Bedrock, HuggingFace, Replicate) |
| `Local` | Localhost without API key (Ollama, LM Studio, llama.cpp) |
| `Custom` | User-defined endpoints |

---

## Stubs

For providers needing significant integration work, ship a stub:

```rust
use crate::ai::providers::openai_compat::{descriptor, StubProvider};

pub struct BedrockProvider;
impl BedrockProvider {
    pub fn new() -> Arc<dyn Provider> {
        Arc::new(StubProvider::new(descriptor(
            "bedrock", "AWS Bedrock", "Managed models on AWS (SigV4 auth).",
            "https://aws.amazon.com/bedrock", ProviderCategory::CloudOther,
            true, false, None, "anthropic.claude-3-5-sonnet-20240620-v1:0",
            &["anthropic.claude-3-5-sonnet-20240620-v1:0"], false,
        )))
    }
}
```

`StubProvider` returns a "not yet implemented" error but appears in the UI so users know it's coming.
