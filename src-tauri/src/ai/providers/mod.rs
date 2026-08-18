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
pub mod anthropic;
pub mod anyscale;
pub mod azure_openai;
pub mod bedrock;
pub mod cohere;
pub mod custom;
pub mod deepinfra;
pub mod deepseek;
pub mod fireworks;
pub mod gemini;
pub mod gpt4all;
pub mod groq;
pub mod huggingface;
pub mod jan;
pub mod koboldcpp;
pub mod llamacpp;
pub mod llamafile;
pub mod lmstudio;
pub mod localai;
pub mod mistral;
pub mod moonshot;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod replicate;
pub mod together;
pub mod vllm;
pub mod yi;
pub mod zhipu;

// === v0.4 generated providers (auto-generated) ===
pub mod abacus;
pub mod abliteration_ai;
pub mod ai302;
pub mod aihubmix;
pub mod alibaba_cn;
pub mod ambient;
pub mod api_airforce;
pub mod avian;
pub mod baseten;
pub mod berget;
pub mod cerebras;
pub mod chutes;
pub mod cortecs;
pub mod crof;
pub mod empiriolabs;
pub mod fastrouter;
pub mod friendli;
pub mod github_copilot;
pub mod helicone;
pub mod hyper;
pub mod impossibl;
pub mod inception;
pub mod inceptron;
pub mod io_net;
pub mod jiekou;
pub mod kenari;
pub mod kilo;
pub mod llmgateway;
pub mod llmtr;
pub mod moark;
pub mod modelscope;
pub mod nano_gpt;
pub mod nearai;
pub mod neuralwatt;
pub mod novita;
pub mod nvidia;
pub mod ofox;
pub mod ollama_cloud;
pub mod opencode_zen;
pub mod orcarouter;
pub mod ovhcloud;
pub mod perplexity;
pub mod pioneer;
pub mod poe;
pub mod qiniu;
pub mod quiver;
pub mod requesty;
pub mod routing_run;
pub mod sakana;
pub mod synthetic;
pub mod tetrate;
pub mod tokenrouter;
pub mod trustedrouter;
pub mod venice;
pub mod vercel;
pub mod wafer_ai;
pub mod wandb;
pub mod xai;
pub mod xpersona;
pub mod zenmux;

pub use aegis_cloud::AegisCloudProvider;
pub use anthropic::AnthropicProvider;
pub use anyscale::AnyscaleProvider;
pub use azure_openai::AzureOpenAiProvider;
pub use bedrock::BedrockProvider;
pub use cohere::CohereProvider;
pub use custom::{
    CustomAnthropicProvider, CustomOllamaProvider, CustomOpenAiProvider, WebhookProvider,
};
pub use deepinfra::DeepInfraProvider;
pub use deepseek::DeepSeekProvider;
pub use fireworks::FireworksProvider;
pub use gemini::GeminiProvider;
pub use gpt4all::Gpt4AllProvider;
pub use groq::GroqProvider;
pub use huggingface::HuggingFaceProvider;
pub use jan::JanProvider;
pub use koboldcpp::KoboldCppProvider;
pub use llamacpp::LlamaCppProvider;
pub use llamafile::LlamafileProvider;
pub use lmstudio::LmStudioProvider;
pub use localai::LocalAiProvider;
pub use mistral::MistralProvider;
pub use moonshot::MoonshotProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use openrouter::OpenRouterProvider;
pub use replicate::ReplicateProvider;
pub use together::TogetherProvider;
pub use vllm::VllmProvider;
pub use yi::YiProvider;
pub use zhipu::ZhipuProvider;

// === v0.4 generated provider re-exports (auto-generated) ===
pub use abacus::AbacusProvider;
pub use abliteration_ai::AbliterationAiProvider;
pub use ai302::Ai302Provider;
pub use aihubmix::AihubmixProvider;
pub use alibaba_cn::AlibabaCnProvider;
pub use ambient::AmbientProvider;
pub use api_airforce::ApiAirforceProvider;
pub use avian::AvianProvider;
pub use baseten::BasetenProvider;
pub use berget::BergetProvider;
pub use cerebras::CerebrasProvider;
pub use chutes::ChutesProvider;
pub use cortecs::CortecsProvider;
pub use crof::CrofProvider;
pub use empiriolabs::EmpiriolabsProvider;
pub use fastrouter::FastrouterProvider;
pub use friendli::FriendliProvider;
pub use github_copilot::GithubCopilotProvider;
pub use helicone::HeliconeProvider;
pub use hyper::HyperProvider;
pub use impossibl::ImpossiblProvider;
pub use inception::InceptionProvider;
pub use inceptron::InceptronProvider;
pub use io_net::IoNetProvider;
pub use jiekou::JiekouProvider;
pub use kenari::KenariProvider;
pub use kilo::KiloProvider;
pub use llmgateway::LlmgatewayProvider;
pub use llmtr::LlmtrProvider;
pub use moark::MoarkProvider;
pub use modelscope::ModelscopeProvider;
pub use nano_gpt::NanoGptProvider;
pub use nearai::NearaiProvider;
pub use neuralwatt::NeuralwattProvider;
pub use novita::NovitaProvider;
pub use nvidia::NvidiaProvider;
pub use ofox::OfoxProvider;
pub use ollama_cloud::OllamaCloudProvider;
pub use opencode_zen::OpencodeZenProvider;
pub use orcarouter::OrcarouterProvider;
pub use ovhcloud::OvhcloudProvider;
pub use perplexity::PerplexityProvider;
pub use pioneer::PioneerProvider;
pub use poe::PoeProvider;
pub use qiniu::QiniuProvider;
pub use quiver::QuiverProvider;
pub use requesty::RequestyProvider;
pub use routing_run::RoutingRunProvider;
pub use sakana::SakanaProvider;
pub use synthetic::SyntheticProvider;
pub use tetrate::TetrateProvider;
pub use tokenrouter::TokenrouterProvider;
pub use trustedrouter::TrustedrouterProvider;
pub use venice::VeniceProvider;
pub use vercel::VercelProvider;
pub use wafer_ai::WaferAiProvider;
pub use wandb::WandbProvider;
pub use xai::XaiProvider;
pub use xpersona::XpersonaProvider;
pub use zenmux::ZenmuxProvider;

/// Shared helper: most cloud providers expose an OpenAI-compatible
/// `/v1/chat/completions` endpoint, so we centralize the HTTP logic here.
pub mod openai_compat;
