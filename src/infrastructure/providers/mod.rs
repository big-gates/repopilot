//! 로컬 provider 실행 공통 모듈.
//! 각 CLI(codex/claude/gemini)를 호출하고 결과/사용량을 표준화한다.

pub mod anthropic;
pub mod gemini;
pub mod openai;
mod command_runner;
mod prompt;
mod usage_parser;

use anyhow::Result;
use async_trait::async_trait;

use crate::domain::review::{ProviderResponse, ReviewRequest};
use crate::infrastructure::config::{Config, command_exists};

pub use command_runner::run_provider_command;
pub use prompt::build_user_prompt;

#[async_trait]
pub trait ReviewProvider: Send + Sync {
    /// 내부 식별자(마커/집계 키)
    fn id(&self) -> &'static str;
    /// 사용자 표시 이름
    fn name(&self) -> &'static str;
    /// 1차 리뷰 실행
    async fn review(&self, request: &ReviewRequest) -> Result<ProviderResponse>;
    /// 임의 프롬프트 실행(2차 상호 코멘트)
    async fn review_prompt(&self, prompt: &str) -> Result<ProviderResponse>;
}

pub fn build_providers(config: &Config) -> Vec<Box<dyn ReviewProvider>> {
    // 커맨드가 실제로 존재하는 provider만 활성화한다.
    let mut providers: Vec<Box<dyn ReviewProvider>> = Vec::new();

    if let Some(provider) = openai::OpenAiProvider::from_config(config) {
        providers.push(Box::new(provider));
    }
    if let Some(provider) = anthropic::AnthropicProvider::from_config(config) {
        providers.push(Box::new(provider));
    }
    if let Some(provider) = gemini::GeminiProvider::from_config(config) {
        providers.push(Box::new(provider));
    }

    providers
}

pub fn command_available(command: &str) -> bool {
    command_exists(command)
}
