//! 애플리케이션 계층이 의존하는 포트(추상 인터페이스) 모음.

use anyhow::Result;
use async_trait::async_trait;

use crate::domain::review::{
    AgentComment, AgentReaction, ProviderResponse, ReviewComment, ReviewRequest, TokenUsage,
};
use crate::domain::target::ReviewTarget;
use crate::infrastructure::config::{Config, HostConfig};

/// 설정 로딩/점검을 담당하는 저장소 포트.
pub trait ConfigRepository: Send + Sync {
    fn load(&self) -> Result<Config>;
    fn inspect_pretty_json(&self) -> Result<String>;
}

/// URL 입력값을 도메인 대상 식별자로 변환하는 포트.
pub trait TargetResolver: Send + Sync {
    fn parse(&self, input: &str) -> Result<ReviewTarget>;
}

/// VCS(GitHub/GitLab) 연동 추상화 포트.
#[async_trait]
pub trait VcsGateway: Send + Sync {
    async fn fetch_head_sha(&self) -> Result<String>;
    async fn fetch_diff(&self, max_bytes: usize) -> Result<String>;
    async fn list_comments(&self) -> Result<Vec<ReviewComment>>;
    async fn create_comment(&self, body: &str) -> Result<ReviewComment>;
    async fn update_comment(&self, comment_id: &str, body: &str) -> Result<ReviewComment>;
}

/// 대상/호스트 설정에 맞는 VCS 게이트웨이를 생성하는 팩토리 포트.
pub trait VcsFactory: Send + Sync {
    fn build(
        &self,
        target: &ReviewTarget,
        host_cfg: Option<&HostConfig>,
        token: Option<String>,
    ) -> Box<dyn VcsGateway>;
}

/// 개별 AI 제공자(에이전트) 실행 포트.
#[async_trait]
pub trait ProviderAgent: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    async fn review(&self, request: &ReviewRequest) -> Result<ProviderResponse>;
    async fn review_prompt(&self, prompt: &str) -> Result<ProviderResponse>;
}

/// 활성화된 제공자 목록을 구성하는 팩토리 포트.
pub trait ProviderFactory: Send + Sync {
    fn build(&self, config: &Config) -> Vec<Box<dyn ProviderAgent>>;
}

/// 리뷰 마크다운 렌더링 포트.
pub trait MarkdownRenderer: Send + Sync {
    fn render_claim(&self, sha: &str, target_url: &str) -> String;
    fn render_agent(&self, sha: &str, target_url: &str, agent: &AgentComment) -> String;
    fn render_final(
        &self,
        sha: &str,
        target_url: &str,
        reactions: &[AgentReaction],
        agent_comment_refs: &[(String, String)],
        usage_rows: &[(String, TokenUsage)],
    ) -> String;
    fn format_usage(&self, usage: &TokenUsage) -> String;
}

/// 콘솔/로그 출력 추상화 포트.
pub trait Reporter: Send + Sync {
    fn section(&self, name: &str);
    fn kv(&self, key: &str, value: &str);
    fn status(&self, scope: &str, message: &str);
    fn provider_status(&self, provider: &str, status: &str, extra: Option<&str>);
    fn raw(&self, line: &str);
}
