//! GPT4All — local OpenAI-compatible desktop runtime.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct Gpt4AllProvider;
impl Gpt4AllProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "gpt4all",
            "GPT4All",
            "Local desktop runtime with built-in OpenAI-compatible server.",
            "https://gpt4all.io",
            ProviderCategory::Local,
            false,
            true,
            Some("http://localhost:4891/v1"),
            "Llama 3 8B Instruct",
            &["Llama 3 8B Instruct", "Mistral 7B Instruct"],
            true,
        ))
    }
}
