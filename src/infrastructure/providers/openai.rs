//! OpenAI/Codex provider 어댑터.

use anyhow::Result;
use async_trait::async_trait;

use crate::domain::review::{ProviderResponse, ReviewRequest};
use crate::infrastructure::config::{Config, ProviderCommandSpec};

use super::{
    ReviewProvider, build_user_prompt, command_available, run_provider_command,
};

pub struct OpenAiProvider {
    spec: ProviderCommandSpec,
}

impl OpenAiProvider {
    /// 설정에서 실행 스펙을 읽고, 명령이 존재할 때만 provider를 활성화한다.
    pub fn from_config(config: &Config) -> Option<Self> {
        let provider = config.providers.openai.as_ref()?;
        let spec = provider.command_spec("codex")?;
        if !command_available(&spec.command) {
            return None;
        }
        Some(Self { spec })
    }
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
        // 공통 프롬프트 형식으로 1차 리뷰를 실행한다.
        let prompt = format!(
            "System instructions:\n{}\n\n{}",
            request.system_prompt,
            build_user_prompt(request)
        );
        run_provider_command(self.name(), &self.spec, &prompt).await
    }

    async fn review_prompt(&self, prompt: &str) -> Result<ProviderResponse> {
        run_provider_command(self.name(), &self.spec, prompt).await
    }
}
