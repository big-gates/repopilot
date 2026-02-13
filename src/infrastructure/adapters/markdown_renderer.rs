//! 마크다운 렌더링 포트 구현 어댑터.

use crate::application::ports::MarkdownRenderer;
use crate::domain::review::{AgentComment, AgentReaction};
use crate::infrastructure::render;

/// 마크다운 렌더링 어댑터.
pub struct MarkdownRendererAdapter;

impl MarkdownRenderer for MarkdownRendererAdapter {
    fn render_claim(&self, sha: &str, target_url: &str) -> String {
        render::render_claim_markdown(sha, target_url)
    }

    fn render_agent(&self, sha: &str, target_url: &str, agent: &AgentComment) -> String {
        render::render_agent_markdown(sha, target_url, agent)
    }

    fn render_final(
        &self,
        sha: &str,
        target_url: &str,
        reactions: &[AgentReaction],
        agent_comment_refs: &[(String, String)],
    ) -> String {
        render::render_final_summary_markdown(sha, target_url, reactions, agent_comment_refs)
    }
}
