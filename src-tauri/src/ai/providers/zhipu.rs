//! Zhipu AI (GLM) — Chinese cloud LLM provider.

use std::sync::Arc;

use crate::ai::provider::{Provider, ProviderCategory};
use crate::ai::providers::openai_compat::{descriptor, make};

pub struct ZhipuProvider;
impl ZhipuProvider {
    pub fn new() -> Arc<dyn Provider> {
        make(descriptor(
            "zhipu",
            "Zhipu AI (GLM)",
            "Chinese cloud LLM provider (GLM-4) with OpenAI-compatible API.",
            "https://open.bigmodel.cn",
            ProviderCategory::CloudOther,
            true,
            false,
            Some("https://open.bigmodel.cn/api/paas/v4"),
            "glm-4-flash",
            &["glm-4", "glm-4-flash", "glm-4-air", "glm-4-long"],
            true,
        ))
    }
}
