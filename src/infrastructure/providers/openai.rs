//! OpenAI/Codex provider 어댑터.

use anyhow::{Result, bail};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};

use crate::domain::review::{ProviderResponse, ReviewRequest, TokenUsage};
use crate::infrastructure::config::{Config, ProviderCommandSpec};

use super::{
    ReviewProvider, build_primary_prompt, command_available, run_provider_command,
    api_runner::{build_api_client, collect_text, send_json},
};

enum OpenAiBackend {
    Api(OpenAiApiBackend),
    Cli(ProviderCommandSpec),
}

struct OpenAiApiBackend {
    client: Client,
    base_url: String,
    model: String,
    credential: String,
}

pub struct OpenAiProvider {
    backend: OpenAiBackend,
}

impl OpenAiProvider {
    /// API key가 있으면 API 모드, 없으면 CLI 모드로 provider를 활성화한다.
    pub fn from_config(config: &Config) -> Option<Self> {
        let provider = config.providers.openai.as_ref()?;
        if !provider.is_enabled() {
            return None;
        }

        if let Some(credential) = provider.resolve_api_key() {
            let api = OpenAiApiBackend {
                client: build_api_client(),
                base_url: provider
                    .api_base
                    .clone()
                    .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
                model: provider
                    .model
                    .clone()
                    .unwrap_or_else(|| "gpt-4.1-mini".to_string()),
                credential,
            };
            return Some(Self {
                backend: OpenAiBackend::Api(api),
            });
        }

        let spec = provider.command_spec("codex")?;
        if !command_available(&spec.command) {
            return None;
        }
        Some(Self {
            backend: OpenAiBackend::Cli(spec),
        })
    }

    async fn review_via_api(&self, prompt: &str) -> Result<ProviderResponse> {
        let OpenAiBackend::Api(api) = &self.backend else {
            bail!("openai api backend is not configured");
        };

        let endpoint = format!(
            "{}/{}",
            api.base_url.trim_end_matches('/'),
            "chat/completions"
        );
        let payload = json!({
            "model": api.model,
            "messages": [
                { "role": "user", "content": prompt }
            ]
        });

        let response = send_json(
            self.name(),
            "request OpenAI API",
            api.client
                .post(endpoint)
                .bearer_auth(&api.credential)
                .json(&payload),
        )
        .await?;

        let content = extract_openai_content(&response).trim().to_string();
        if content.is_empty() {
            bail!("OpenAI/Codex: empty response content");
        }

        Ok(ProviderResponse {
            content,
            usage: TokenUsage {
                prompt_tokens: response
                    .pointer("/usage/prompt_tokens")
                    .and_then(Value::as_u64),
                completion_tokens: response
                    .pointer("/usage/completion_tokens")
                    .or_else(|| response.pointer("/usage/output_tokens"))
                    .and_then(Value::as_u64),
                total_tokens: response
                    .pointer("/usage/total_tokens")
                    .and_then(Value::as_u64),
            },
        })
    }
}

fn extract_openai_content(response: &Value) -> String {
    if let Some(first_choice) = response
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        && let Some(message) = first_choice.get("message")
    {
        return collect_text(message);
    }

    if let Some(output_text) = response.get("output_text").and_then(Value::as_str) {
        return output_text.to_string();
    }

    String::new()
}

#[async_trait]
impl ReviewProvider for OpenAiProvider {
    fn id(&self) -> &'static str {
        "openai"
    }

    fn name(&self) -> &'static str {
        "OpenAI/Codex"
    }

    async fn review(&self, request: &ReviewRequest) -> Result<ProviderResponse> {
        let prompt = build_primary_prompt(request);
        match &self.backend {
            OpenAiBackend::Api(_) => self.review_via_api(&prompt).await,
            OpenAiBackend::Cli(spec) => run_provider_command(self.name(), spec, &prompt).await,
        }
    }

    async fn review_prompt(&self, prompt: &str) -> Result<ProviderResponse> {
        match &self.backend {
            OpenAiBackend::Api(_) => self.review_via_api(prompt).await,
            OpenAiBackend::Cli(spec) => run_provider_command(self.name(), spec, prompt).await,
        }
    }
}
