//! Yi (01.AI) — Chinese cloud LLM provider.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct YiProvider;
impl YiProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "yi",
            "Yi (01.AI)",
            "Chinese cloud LLM provider (Yi) with OpenAI-compatible API.",
            "https://platform.lingyiwanwu.com",
            ProviderCategory::CloudOther,
            true,
            false,
            Some("https://api.lingyiwanwu.com/v1"),
            "yi-large",
            &["yi-large", "yi-medium", "yi-light"],
            true,
        ))
    }
}
