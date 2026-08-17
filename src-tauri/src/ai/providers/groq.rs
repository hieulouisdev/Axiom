//! Groq — Llama / Mixtral with ultra-low latency on GroqCloud.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct GroqProvider;
impl GroqProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "groq",
            "Groq",
            "Llama / Mixtral with ultra-low latency on GroqCloud.",
            "https://groq.com",
            ProviderCategory::CloudMajor,
            true,
            false,
            Some("https://api.groq.com/openai/v1"),
            "llama-3.3-70b-versatile",
            &["llama-3.3-70b-versatile", "llama-3.1-8b-instant", "mixtral-8x7b-32768"],
            true,
        ))
    }
}
