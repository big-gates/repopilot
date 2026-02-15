//! Google Gemini provider 어댑터.

use anyhow::{Result, bail};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::{Value, json};
use url::Url;

use crate::domain::review::{ProviderResponse, ReviewRequest, TokenUsage};
use crate::infrastructure::config::{Config, ProviderCommandSpec, resolve_provider_api_key};

use super::{
    ReviewProvider, build_primary_prompt, command_available, run_provider_command,
    api_runner::{build_api_client, collect_text, send_json},
};

struct CliBackend {
    spec: ProviderCommandSpec,
    auth_command: Option<Vec<String>>,
    auto_auth: bool,
}

enum GeminiBackend {
    Api(GeminiApiBackend),
    Cli(CliBackend),
}

struct GeminiApiBackend {
    client: Client,
    base_url: String,
    model: String,
    credential: String,
}

pub struct GeminiProvider {
    backend: GeminiBackend,
}

impl GeminiProvider {
    /// API key가 있으면 API 모드, 없으면 CLI 모드로 provider를 활성화한다.
    pub fn from_config(config: &Config) -> Option<Self> {
        let provider = config.providers.gemini.as_ref()?;
        if !provider.is_enabled() {
            return None;
        }

        if let Some(credential) = resolve_provider_api_key(provider).credential {
            let api = GeminiApiBackend {
                client: build_api_client(),
                base_url: provider
                    .api_base
                    .clone()
                    .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string()),
                model: provider
                    .model
                    .clone()
                    .unwrap_or_else(|| "gemini-2.0-flash".to_string()),
                credential,
            };
            return Some(Self {
                backend: GeminiBackend::Api(api),
            });
        }

        let spec = provider.command_spec("gemini")?;
        if !command_available(&spec.command) {
            return None;
        }

        let auth_command = provider
            .auth_command
            .clone()
            .filter(|v| !v.is_empty())
            .or_else(|| Some(vec![spec.command.clone(), "login".to_string()]));

        Some(Self {
            backend: GeminiBackend::Cli(CliBackend {
                spec,
                auth_command,
                auto_auth: provider.auto_auth(),
            }),
        })
    }

    async fn review_via_api(&self, prompt: &str) -> Result<ProviderResponse> {
        let GeminiBackend::Api(api) = &self.backend else {
            bail!("gemini api backend is not configured");
        };

        let endpoint = format!(
            "{}/models/{}:generateContent",
            api.base_url.trim_end_matches('/'),
            api.model
        );
        let payload = json!({
            "contents": [
                {
                    "parts": [
                        { "text": prompt }
                    ]
                }
            ]
        });

        // Gemini는 API key(query) 또는 OAuth(Bearer) 방식 모두 허용한다.
        let response = if api.credential.starts_with("AIza") {
            let mut url = Url::parse(&endpoint)?;
            url.query_pairs_mut().append_pair("key", &api.credential);
            send_json(
                self.name(),
                "request Gemini API",
                api.client.post(url).json(&payload),
            )
            .await?
        } else {
            send_json(
                self.name(),
                "request Gemini API",
                api.client
                    .post(endpoint)
                    .bearer_auth(&api.credential)
                    .json(&payload),
            )
            .await?
        };

        let content = extract_gemini_content(&response).trim().to_string();
        if content.is_empty() {
            bail!("Gemini: empty response content");
        }

        Ok(ProviderResponse {
            content,
            usage: TokenUsage {
                prompt_tokens: response
                    .pointer("/usageMetadata/promptTokenCount")
                    .and_then(Value::as_u64),
                completion_tokens: response
                    .pointer("/usageMetadata/candidatesTokenCount")
                    .and_then(Value::as_u64),
                total_tokens: response
                    .pointer("/usageMetadata/totalTokenCount")
                    .and_then(Value::as_u64),
            },
        })
    }
}

fn extract_gemini_content(response: &Value) -> String {
    if let Some(content) = response.pointer("/candidates/0/content") {
        return collect_text(content);
    }
    String::new()
}

#[async_trait]
impl ReviewProvider for GeminiProvider {
    fn id(&self) -> &'static str {
        "gemini"
    }

    fn name(&self) -> &'static str {
        "Gemini"
    }

    async fn review(&self, request: &ReviewRequest) -> Result<ProviderResponse> {
        let prompt = build_primary_prompt(request);
        match &self.backend {
            GeminiBackend::Api(_) => self.review_via_api(&prompt).await,
            GeminiBackend::Cli(cli) => {
                run_provider_command(
                    self.name(),
                    &cli.spec,
                    &prompt,
                    cli.auth_command.as_deref(),
                    cli.auto_auth,
                )
                .await
            }
        }
    }

    async fn review_prompt(&self, prompt: &str) -> Result<ProviderResponse> {
        match &self.backend {
            GeminiBackend::Api(_) => self.review_via_api(prompt).await,
            GeminiBackend::Cli(cli) => {
                run_provider_command(
                    self.name(),
                    &cli.spec,
                    prompt,
                    cli.auth_command.as_deref(),
                    cli.auto_auth,
                )
                .await
            }
        }
    }
}
