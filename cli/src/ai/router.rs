//! AI router — picks the active provider and forwards chat requests.

use std::sync::Arc;

use crate::ai::provider::{ChatRequest, ChatResponse, Provider, ProviderRegistry};

pub struct AiRouter {
    pub registry: Arc<parking_lot::Mutex<ProviderRegistry>>,
}

impl AiRouter {
    pub fn new(registry: ProviderRegistry) -> Self {
        Self { registry: Arc::new(parking_lot::Mutex::new(registry)) }
    }

    pub fn active_provider_id(&self) -> Option<String> {
        self.registry.lock().active_id().map(str::to_string)
    }

    pub async fn chat(&self, req: &ChatRequest, provider_override: Option<&str>) -> anyhow::Result<ChatResponse> {
        let provider_id = match provider_override {
            Some(p) => p.to_string(),
            None => {
                let reg = self.registry.lock();
                reg.active_id()
                    .ok_or_else(|| anyhow::anyhow!("no active provider configured — run `aegis configure <provider> --key <KEY>` first"))?
                    .to_string()
            }
        };
        let provider = self.registry.lock().build(&provider_id)?;
        provider.chat(req).await
    }
}
