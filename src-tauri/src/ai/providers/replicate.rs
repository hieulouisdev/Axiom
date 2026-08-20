//! Replicate — hosted open-source models with Prediction API.
//!
//! Phase 2: Full implementation of the async Prediction API.
//! POST to create prediction → poll until succeeded/failed → return output.

use parking_lot::RwLock;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::ai::provider::{
    ChatMessage, ChatRequest, ChatResponse, Provider, ProviderCategory, ProviderCreds,
    ProviderDescriptor, Role,
};
use crate::error::{AegisError, Result};

const REPLICATE_API_BASE: &str = "https://api.replicate.com/v1";

pub struct ReplicateProvider {
    descriptor: ProviderDescriptor,
    creds: RwLock<ProviderCreds>,
    client: Client,
}

impl ReplicateProvider {
    pub fn new() -> std::sync::Arc<dyn Provider> {
        let desc = ProviderDescriptor {
            id: "replicate".into(),
            name: "Replicate".into(),
            description: "Hosted open-source models via Prediction API (async).".into(),
            homepage: "https://replicate.com".into(),
            category: ProviderCategory::CloudOther,
            requires_api_key: true,
            local: false,
            default_base_url: None,
            default_model: "meta/llama-3.3-70b-instruct".into(),
            known_models: vec![
                "meta/llama-3.3-70b-instruct".into(),
                "mistralai/mistral-7b-instruct-v0.3".into(),
                "meta/meta-llama-3.1-405b-instruct".into(),
            ],
            implemented: true,
        };
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // Long timeout for polling
            .build()
            .expect("reqwest client");
        std::sync::Arc::new(Self {
            descriptor: desc,
            creds: RwLock::new(ProviderCreds::default()),
            client,
        })
    }

    fn creds(&self) -> ProviderCreds {
        self.creds.read().clone()
    }

    /// Create a prediction on Replicate.
    async fn create_prediction(
        &self,
        model: &str,
        prompt: &str,
        api_key: &str,
        system_prompt: Option<&str>,
    ) -> Result<String> {
        let url = format!("{REPLICATE_API_BASE}/predictions");

        let mut input = json!({
            "prompt": prompt,
        });
        if let Some(sys) = system_prompt {
            input["system_prompt"] = json!(sys);
        }

        let body = json!({
            "version": model,
            "input": input,
        });

        let resp = self
            .client
            .post(&url)
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AegisError::Ai(format!(
                "HTTP {status} creating Replicate prediction: {text}"
            )));
        }

        let resp_body: serde_json::Value = resp.json().await?;
        let prediction_id = resp_body["id"]
            .as_str()
            .ok_or_else(|| AegisError::Ai("no prediction id in response".into()))?;

        Ok(prediction_id.to_string())
    }

    /// Poll a prediction until it completes or fails.
    async fn poll_prediction(
        &self,
        prediction_id: &str,
        api_key: &str,
    ) -> Result<serde_json::Value> {
        let url = format!("{REPLICATE_API_BASE}/predictions/{prediction_id}");
        let max_attempts = 120; // 120 * 2s = 4 minutes max wait

        for _ in 0..max_attempts {
            let resp = self.client.get(&url).bearer_auth(api_key).send().await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                return Err(AegisError::Ai(format!(
                    "HTTP {status} polling prediction: {text}"
                )));
            }

            let body: serde_json::Value = resp.json().await?;
            let status = body["status"].as_str().unwrap_or("unknown");

            match status {
                "succeeded" => return Ok(body),
                "failed" | "canceled" => {
                    let error = body["error"].as_str().unwrap_or("unknown error");
                    return Err(AegisError::Ai(format!(
                        "Replicate prediction {status}: {error}"
                    )));
                }
                "starting" | "processing" => {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                _ => {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }

        Err(AegisError::Ai("Replicate prediction timed out".into()))
    }
}

#[async_trait]
impl Provider for ReplicateProvider {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    fn set_creds(&self, creds: ProviderCreds) {
        *self.creds.write() = creds;
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let creds = self.creds();
        let api_key = creds
            .api_key
            .clone()
            .ok_or_else(|| AegisError::AiNotConfigured("replicate requires an API key".into()))?;

        let model = req
            .model
            .clone()
            .or(creds.model.clone())
            .unwrap_or_else(|| self.descriptor.default_model.clone());

        // Build prompt from messages
        let mut system_prompt = String::new();
        let mut conversation = String::new();

        for m in &req.messages {
            match m.role {
                Role::System => {
                    system_prompt = m.content.clone();
                }
                Role::User => {
                    if !conversation.is_empty() {
                        conversation.push('\n');
                    }
                    conversation.push_str(&format!("User: {}", m.content));
                }
                Role::Assistant => {
                    if !conversation.is_empty() {
                        conversation.push('\n');
                    }
                    conversation.push_str(&format!("Assistant: {}", m.content));
                }
                Role::Tool => {}
            }
        }

        // If the last message is from user, use it as the prompt
        let prompt = req
            .messages
            .iter()
            .rev()
            .find(|m| m.role == Role::User)
            .map(|m| m.content.clone())
            .unwrap_or(conversation);

        // Create prediction
        let prediction_id = self
            .create_prediction(
                &model,
                &prompt,
                &api_key,
                if system_prompt.is_empty() {
                    None
                } else {
                    Some(&system_prompt)
                },
            )
            .await?;

        // Poll for result
        let result = self.poll_prediction(&prediction_id, &api_key).await?;

        // Extract output
        let output_owned: String = if let Some(s) = result["output"].as_str() {
            s.to_string()
        } else if let Some(arr) = result["output"].as_array() {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join("")
        } else {
            String::new()
        };

        Ok(ChatResponse {
            message: ChatMessage::assistant(output_owned),
            model,
            usage: None, // Replicate doesn't report token usage in the same way
        })
    }

    async fn ping(&self) -> Result<()> {
        // Simple test: verify the API key works by checking account
        let creds = self.creds();
        let api_key = creds
            .api_key
            .clone()
            .ok_or_else(|| AegisError::AiNotConfigured("replicate requires an API key".into()))?;

        let resp = self
            .client
            .get(format!("{REPLICATE_API_BASE}/predictions?limit=1"))
            .bearer_auth(&api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(AegisError::Ai(format!("Replicate ping: HTTP {status}")));
        }

        Ok(())
    }
}
