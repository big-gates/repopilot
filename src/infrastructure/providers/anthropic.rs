//! Anthropic/Claude provider 어댑터.

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

enum AnthropicBackend {
    Api(AnthropicApiBackend),
    Cli(ProviderCommandSpec),
}

struct AnthropicApiBackend {
    client: Client,
    base_url: String,
    model: String,
    credential: String,
}

pub struct AnthropicProvider {
    backend: AnthropicBackend,
}

impl AnthropicProvider {
    /// API key가 있으면 API 모드, 없으면 CLI 모드로 provider를 활성화한다.
    pub fn from_config(config: &Config) -> Option<Self> {
        let provider = config.providers.anthropic.as_ref()?;
        if !provider.is_enabled() {
            return None;
        }

        if let Some(credential) = provider.resolve_api_key() {
            let api = AnthropicApiBackend {
                client: build_api_client(),
                base_url: provider
                    .api_base
                    .clone()
                    .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string()),
                model: provider
                    .model
                    .clone()
                    .unwrap_or_else(|| "claude-3-7-sonnet-latest".to_string()),
                credential,
            };
            return Some(Self {
                backend: AnthropicBackend::Api(api),
            });
        }

        let spec = provider.command_spec("claude")?;
        if !command_available(&spec.command) {
            return None;
        }
        Some(Self {
            backend: AnthropicBackend::Cli(spec),
        })
    }

    async fn review_via_api(&self, prompt: &str) -> Result<ProviderResponse> {
        let AnthropicBackend::Api(api) = &self.backend else {
            bail!("anthropic api backend is not configured");
        };

        let endpoint = format!("{}/{}", api.base_url.trim_end_matches('/'), "messages");
        let payload = json!({
            "model": api.model,
            "max_tokens": 4096,
            "messages": [
                { "role": "user", "content": prompt }
            ]
        });

        // Anthropic API key(sk-ant-...)와 OAuth/Bearer 토큰을 모두 수용한다.
        let request = if api.credential.starts_with("sk-ant-") {
            api.client
                .post(endpoint)
                .header("x-api-key", &api.credential)
                .header("anthropic-version", "2023-06-01")
                .json(&payload)
        } else {
            api.client
                .post(endpoint)
                .bearer_auth(&api.credential)
                .header("anthropic-version", "2023-06-01")
                .json(&payload)
        };

        let response = send_json(self.name(), "request Anthropic API", request).await?;
        let content = extract_anthropic_content(&response).trim().to_string();
        if content.is_empty() {
            bail!("Claude: empty response content");
        }

        Ok(ProviderResponse {
            content,
            usage: TokenUsage {
                prompt_tokens: response
                    .pointer("/usage/input_tokens")
                    .and_then(Value::as_u64),
                completion_tokens: response
                    .pointer("/usage/output_tokens")
                    .and_then(Value::as_u64),
                total_tokens: match (
                    response.pointer("/usage/input_tokens").and_then(Value::as_u64),
                    response.pointer("/usage/output_tokens").and_then(Value::as_u64),
                ) {
                    (Some(input), Some(output)) => Some(input + output),
                    (Some(input), None) => Some(input),
                    (None, Some(output)) => Some(output),
                    (None, None) => None,
                },
            },
        })
    }
}

fn extract_anthropic_content(response: &Value) -> String {
    if let Some(content) = response.get("content") {
        return collect_text(content);
    }
    String::new()
}

#[async_trait]
impl ReviewProvider for AnthropicProvider {
    fn id(&self) -> &'static str {
        "anthropic"
    }

    fn name(&self) -> &'static str {
        "Claude"
    }

    async fn review(&self, request: &ReviewRequest) -> Result<ProviderResponse> {
        let prompt = build_primary_prompt(request);
        match &self.backend {
            AnthropicBackend::Api(_) => self.review_via_api(&prompt).await,
            AnthropicBackend::Cli(spec) => run_provider_command(self.name(), spec, &prompt).await,
        }
    }

    async fn review_prompt(&self, prompt: &str) -> Result<ProviderResponse> {
        match &self.backend {
            AnthropicBackend::Api(_) => self.review_via_api(prompt).await,
            AnthropicBackend::Cli(spec) => run_provider_command(self.name(), spec, prompt).await,
        }
    }
}
