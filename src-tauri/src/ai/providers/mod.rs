//! All builtin AI provider implementations.
//!
//! Most modern providers (DeepSeek, Groq, OpenRouter, Together, …) expose an
//! OpenAI-compatible `/v1/chat/completions` endpoint. We share the
//! [`openai_compat`] helper to avoid duplicating HTTP boilerplate, and only
//! write bespoke clients for providers whose API shape differs materially
//! (Anthropic Messages API, Google Gemini generateContent, Ollama native API).
//!
//! v0.3 introduces [`aegis_cloud`] — the built-in zero-config provider backed
//! by Z.AI GLM-4.6 that auto-loads an API key from the `AEGIS_DEFAULT_API_KEY`
//! environment variable. It is the first provider in the registry and the
//! default active provider when no other has been configured.

pub mod aegis_cloud;
pub mod openai;
pub mod anthropic;
pub mod gemini;
pub mod deepseek;
pub mod groq;
pub mod openrouter;
pub mod mistral;
pub mod cohere;
pub mod together;
pub mod anyscale;
pub mod ollama;
pub mod lmstudio;
pub mod localai;
pub mod llamacpp;
pub mod gpt4all;
pub mod jan;
pub mod koboldcpp;
pub mod vllm;
pub mod llamafile;
pub mod azure_openai;
pub mod bedrock;
pub mod huggingface;
pub mod replicate;
pub mod moonshot;
pub mod zhipu;
pub mod yi;
pub mod deepinfra;
pub mod fireworks;
pub mod custom;

pub use aegis_cloud::AegisCloudProvider;
pub use openai::OpenAiProvider;
pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use deepseek::DeepSeekProvider;
pub use groq::GroqProvider;
pub use openrouter::OpenRouterProvider;
pub use mistral::MistralProvider;
pub use cohere::CohereProvider;
pub use together::TogetherProvider;
pub use anyscale::AnyscaleProvider;
pub use ollama::OllamaProvider;
pub use lmstudio::LmStudioProvider;
pub use localai::LocalAiProvider;
pub use llamacpp::LlamaCppProvider;
pub use gpt4all::Gpt4AllProvider;
pub use jan::JanProvider;
pub use koboldcpp::KoboldCppProvider;
pub use vllm::VllmProvider;
pub use llamafile::LlamafileProvider;
pub use azure_openai::AzureOpenAiProvider;
pub use bedrock::BedrockProvider;
pub use huggingface::HuggingFaceProvider;
pub use replicate::ReplicateProvider;
pub use moonshot::MoonshotProvider;
pub use zhipu::ZhipuProvider;
pub use yi::YiProvider;
pub use deepinfra::DeepInfraProvider;
pub use fireworks::FireworksProvider;
pub use custom::{CustomOpenAiProvider, CustomAnthropicProvider, CustomOllamaProvider, WebhookProvider};

/// Shared helper: most cloud providers expose an OpenAI-compatible
/// `/v1/chat/completions` endpoint, so we centralize the HTTP logic here.
pub mod openai_compat;
