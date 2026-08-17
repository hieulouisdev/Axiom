# Adding a new AI provider

This guide walks through adding a new AI provider to Aegis AI.

## When to use the OpenAI-compatible shortcut

If your provider exposes an OpenAI-compatible `/v1/chat/completions` endpoint
(DeepSeek, Groq, OpenRouter, Mistral, Together, Anyscale, Moonshot, Zhipu,
Yi, DeepInfra, Fireworks, HuggingFace, LM Studio, LocalAI, llama.cpp, GPT4All,
Jan, KoboldCpp, vLLM, Llamafile all do), you can add it in **one file, ~15
lines**.

### Step 1: create the provider file

Create `src-tauri/src/ai/providers/<your_provider>.rs`:

```rust
//! <YourProvider> — <short description>.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct YourProvider;
impl YourProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "your_provider",                                  // unique id
            "Your Provider Display Name",                     // display name
            "Short description for the settings UI.",          // description
            "https://your-provider.com",                       // homepage
            ProviderCategory::CloudMajor,                     // or CloudOther / Local / Custom
            true,                                             // requires_api_key
            false,                                            // local?
            Some("https://api.your-provider.com/v1"),          // default base URL
            "your-default-model",                             // default model
            &["model-a", "model-b", "model-c"],               // known models
            true,                                             // implemented
        ))
    }
}
```

### Step 2: register it

Edit `src-tauri/src/ai/providers/mod.rs`:

```rust
pub mod your_provider;
pub use your_provider::YourProvider;
```

Edit `src-tauri/src/ai/provider.rs` and add to `ProviderRegistry::with_builtin`:

```rust
register(&mut map, YourProvider::new());
```

That's it. The provider will appear in the **AI Providers** panel
automatically.

## When to write a bespoke client

Write a bespoke client when the provider's API shape differs materially from
OpenAI's. Examples:

- **Anthropic**: separates the system message from the conversation; uses
  `x-api-key` + `anthropic-version` headers.
- **Google Gemini**: uses `:generateContent?key=…` URL; `parts` / `roles`
  are shaped differently.
- **Ollama native API**: `/api/chat` with `options.system` instead of a
  system role message.
- **AWS Bedrock**: requires SigV4 request signing.
- **Replicate**: uses async predictions (POST → poll → fetch output).

### Step 1: implement the `Provider` trait

Create `src-tauri/src/ai/providers/<your_provider>.rs`:

```rust
use std::sync::RwLock;
use async_trait::async_trait;
use reqwest::Client;
use crate::ai::provider::{
    ChatRequest, ChatResponse, Provider, ProviderCategory,
    ProviderCreds, ProviderDescriptor,
};
use crate::error::Result;

pub struct YourProvider {
    descriptor: ProviderDescriptor,
    creds: RwLock<ProviderCreds>,
    client: Client,
}

impl YourProvider {
    pub fn new() -> std::sync::Arc<Self> {
        // ... build descriptor, build client, return Arc::new(...)
    }
}

#[async_trait]
impl Provider for YourProvider {
    fn descriptor(&self) -> &ProviderDescriptor { &self.descriptor }
    fn set_creds(&self, creds: ProviderCreds) {
        *self.creds.write().unwrap() = creds;
    }
    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        // ... your HTTP request ...
    }
    async fn ping(&self) -> Result<()> {
        // ... lightweight connectivity test ...
    }
}
```

### Step 2: register it (same as above)

## Categories

Pick the right category for your provider:

| Category | Use for |
|---|---|
| `CloudMajor` | The big well-known clouds (OpenAI, Anthropic, Gemini, DeepSeek, Groq, Mistral, Cohere, Together, Anyscale, OpenRouter) |
| `CloudOther` | Smaller or regional clouds (Azure OpenAI, Bedrock, HuggingFace, Replicate, Moonshot, Zhipu, Yi, DeepInfra, Fireworks) |
| `Local` | Anything that runs on `localhost` without an API key (Ollama, LM Studio, etc.) |
| `Custom` | User-defined endpoints (custom OpenAI-compat, webhook, etc.) |

## Stubs vs. real implementations

If your provider requires significant integration work (e.g. SigV4 signing
for Bedrock), it's fine to ship a stub for v0.1 and implement it in Phase 2:

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

The `StubProvider` returns a friendly "not yet implemented in v0.1" error
when called. It still appears in the UI so users know it's coming.
