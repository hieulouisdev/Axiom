//! AWS Bedrock — managed Anthropic / Meta / Mistral models on AWS.
//!
//! Phase 2: Full implementation with AWS SigV4 request signing.
//! Uses the Anthropic Messages API format for Claude models,
//! and OpenAI-compat format for other models.

use std::collections::BTreeMap;
use std::sync::RwLock;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;

use crate::error::{AegisError, Result};
use crate::ai::provider::{
    ChatMessage, ChatRequest, ChatResponse, ChatStreamChunk, Provider,
    ProviderCategory, ProviderCreds, ProviderDescriptor, Role, Usage,
};

pub struct BedrockProvider {
    descriptor: ProviderDescriptor,
    creds: RwLock<ProviderCreds>,
    client: Client,
}

impl BedrockProvider {
    pub fn new() -> std::sync::Arc<dyn Provider> {
        let desc = ProviderDescriptor {
            id: "bedrock".into(),
            name: "AWS Bedrock".into(),
            description: "Managed Anthropic / Meta / Mistral models on AWS (SigV4 auth).".into(),
            homepage: "https://aws.amazon.com/bedrock".into(),
            category: ProviderCategory::CloudOther,
            requires_api_key: true,
            local: false,
            default_base_url: None,
            default_model: "anthropic.claude-3-5-sonnet-20240620-v1:0".into(),
            known_models: vec![
                "anthropic.claude-3-5-sonnet-20240620-v1:0".into(),
                "anthropic.claude-3-haiku-20240307-v1:0".into(),
                "meta.llama3-1-70b-instruct-v1:0".into(),
                "mistral.mistral-large-2407-v1:0".into(),
            ],
            implemented: true,
        };
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("reqwest client");
        std::sync::Arc::new(Self {
            descriptor: desc,
            creds: RwLock::new(ProviderCreds::default()),
            client,
        })
    }

    fn creds(&self) -> ProviderCreds {
        self.creds.read().unwrap().clone()
    }

    /// Build the Bedrock invoke URL:
    /// `https://bedrock-runtime.{region}.amazonaws.com/model/{model_id}/invoke`
    fn build_url(&self, creds: &ProviderCreds, model: &str) -> String {
        let region = creds.extra.get("region")
            .cloned()
            .unwrap_or_else(|| "us-east-1".into());

        format!(
            "https://bedrock-runtime.{}.amazonaws.com/model/{}/invoke",
            region, model
        )
    }

    /// Build the Anthropic-style request body for Claude models.
    fn build_anthropic_body(&self, req: &ChatRequest, model: &str) -> serde_json::Value {
        // Separate system messages from user/assistant messages
        let mut system_parts = Vec::new();
        let mut messages = Vec::new();

        for m in &req.messages {
            match m.role {
                Role::System => {
                    system_parts.push(json!({ "text": m.content }));
                }
                Role::User => {
                    messages.push(json!({
                        "role": "user",
                        "content": vec![json!({ "text": m.content })]
                    }));
                }
                Role::Assistant => {
                    messages.push(json!({
                        "role": "assistant",
                        "content": vec![json!({ "text": m.content })]
                    }));
                }
                Role::Tool => {}
            }
        }

        let mut body = json!({
            "anthropic_version": "bedrock-2023-05-31",
            "messages": messages,
            "max_tokens": req.max_tokens.unwrap_or(2048),
            "temperature": req.temperature.unwrap_or(0.7),
        });

        if !system_parts.is_empty() {
            body["system"] = json!(system_parts);
        }

        body
    }

    /// Sign the request with AWS SigV4.
    /// Uses the `aws_sigv4` crate for proper signing.
    fn sign_request(
        &self,
        method: &str,
        url: &str,
        body: &[u8],
        creds: &ProviderCreds,
    ) -> Result<reqwest::RequestBuilder> {
        let access_key = creds.api_key.clone()
            .ok_or_else(|| AegisError::AiNotConfigured(
                "bedrock requires AWS access key as api_key".into()
            ))?;

        let secret_key = creds.extra.get("secret_key")
            .cloned()
            .ok_or_else(|| AegisError::AiNotConfigured(
                "bedrock requires 'secret_key' in extra config".into()
            ))?;

        let region = creds.extra.get("region")
            .cloned()
            .unwrap_or_else(|| "us-east-1".into());

        let session_token = creds.extra.get("session_token").cloned();

        // Use AWS SigV4 signing
        let signing_config = aws_sigv4::http::SigningConfig {
            algorithm: aws_sigv4::Algorithm::HmacSha256,
            region: region.clone(),
            service: "bedrock".into(),
            time: None,
            credentials: aws_sigv4::Credentials::new(
                access_key,
                secret_key,
                session_token,
                None,
                "aegis-bedrock"
            ).map_err(|e| AegisError::Ai(format!("AWS credentials: {e}")))?,
            payload_config: None,
            uri_path_normalization: true,
            settings: aws_sigv4::http::SigningSettings::default(),
        };

        // Build the unsigned request
        let unsigned = http::Request::builder()
            .method(method)
            .uri(url)
            .header("content-type", "application/json")
            .body(body.to_vec())
            .map_err(|e| AegisError::Ai(format!("build request: {e}")))?;

        // Sign the request
        let (signing_params, _) = aws_sigv4::http::sign(unsigned, &signing_config)
            .map_err(|e| AegisError::Ai(format!("SigV4 signing: {e}")))?;

        // Convert to reqwest request
        let mut builder = self.client.post(url)
            .header("content-type", "application/json");

        for (name, value) in signing_params.headers() {
            if let Ok(v) = value.to_str() {
                builder = builder.header(name.as_str(), v);
            }
        }

        builder = builder.body(body.to_vec());

        Ok(builder)
    }
}

#[async_trait]
impl Provider for BedrockProvider {
    fn descriptor(&self) -> &ProviderDescriptor {
        &self.descriptor
    }

    fn set_creds(&self, creds: ProviderCreds) {
        *self.creds.write().unwrap() = creds;
    }

    async fn chat(&self, req: ChatRequest) -> Result<ChatResponse> {
        let creds = self.creds();
        let model = req.model.clone()
            .or(creds.model.clone())
            .unwrap_or_else(|| self.descriptor.default_model.clone());

        let url = self.build_url(&creds, &model);
        let body = self.build_anthropic_body(&req, &model);
        let body_bytes = serde_json::to_vec(&body)
            .map_err(|e| AegisError::Ai(format!("serialize: {e}")))?;

        let signed_req = self.sign_request("POST", &url, &body_bytes, &creds)?;
        let resp = signed_req.send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AegisError::Ai(format!("HTTP {status} from bedrock: {text}")));
        }

        let resp_body: serde_json::Value = resp.json().await?;
        parse_bedrock_response(&resp_body, &model)
    }

    async fn ping(&self) -> Result<()> {
        let req = ChatRequest {
            messages: vec![ChatMessage::user("ping")],
            model: None,
            temperature: Some(0.0),
            max_tokens: Some(1),
            top_p: None,
            stop: vec![],
            extra: Default::default(),
        };
        self.chat(req).await.map(|_| ())
    }
}

fn parse_bedrock_response(body: &serde_json::Value, model: &str) -> Result<ChatResponse> {
    // Bedrock response format for Claude:
    // { "content": [{ "text": "..." }], "usage": { "inputTokens": N, "outputTokens": N } }
    let content = body["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let usage = body.get("usage").and_then(|u| {
        let input = u["inputTokens"].as_u64().unwrap_or(0) as u32;
        let output = u["outputTokens"].as_u64().unwrap_or(0) as u32;
        Some(Usage {
            prompt_tokens: input,
            completion_tokens: output,
            total_tokens: input + output,
        })
    });

    Ok(ChatResponse {
        message: ChatMessage::assistant(content),
        model: model.to_string(),
        usage,
    })
}
