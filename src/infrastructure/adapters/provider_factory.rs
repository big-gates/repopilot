//! Provider 포트 구현 어댑터.

use anyhow::Result;
use async_trait::async_trait;

use crate::application::ports::{ProviderAgent, ProviderFactory};
use crate::domain::review::{ProviderResponse, ReviewRequest};
use crate::infrastructure::{config, providers};

/// Provider 팩토리 어댑터.
pub struct ProviderFactoryAdapter;

impl ProviderFactory for ProviderFactoryAdapter {
    fn build(&self, config: &config::Config) -> Vec<Box<dyn ProviderAgent>> {
        providers::build_providers(config)
            .into_iter()
            .map(|inner| Box::new(ProviderAgentAdapter { inner }) as Box<dyn ProviderAgent>)
            .collect()
    }
}

/// 인프라 Provider를 애플리케이션 포트로 감싸는 래퍼.
struct ProviderAgentAdapter {
    inner: Box<dyn providers::ReviewProvider>,
}

#[async_trait]
impl ProviderAgent for ProviderAgentAdapter {
    fn id(&self) -> &'static str {
        self.inner.id()
    }

    fn name(&self) -> &'static str {
        self.inner.name()
    }

    async fn review(&self, request: &ReviewRequest) -> Result<ProviderResponse> {
        self.inner.review(request).await
    }

    async fn review_prompt(&self, prompt: &str) -> Result<ProviderResponse> {
        self.inner.review_prompt(prompt).await
    }
}
