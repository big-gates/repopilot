//! VCS 코멘트용 Markdown 렌더링 모듈.

use crate::domain::review::{AgentComment, AgentReaction};

/// 리뷰 시작 상태를 나타내는 claim 코멘트 본문을 생성한다.
pub fn render_claim_markdown(sha: &str, target_url: &str) -> String {
    format!(
        "<!-- repopilot-bot claim sha={sha} -->\n\n# Multi-Agent Code Review\n\n- Target: {target_url}\n- Head SHA: `{sha}`\n\nReview in progress..."
    )
}

/// 에이전트별 개별 코멘트 본문을 생성한다.
pub fn render_agent_markdown(sha: &str, target_url: &str, agent: &AgentComment) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "<!-- repopilot-bot agent={} sha={} -->\n\n",
        agent.provider_id, sha
    ));
    out.push_str(&format!("# Agent Review: {}\n\n", agent.provider_name));
    out.push_str(&format!("- Target: {}\n", target_url));
    out.push_str(&format!("- Head SHA: `{}`\n", sha));
    out.push('\n');
    out.push_str(agent.body.trim());
    out.push('\n');
    out
}

/// 최종 요약 코멘트(상호 코멘트)를 생성한다.
pub fn render_final_summary_markdown(
    sha: &str,
    target_url: &str,
    reactions: &[AgentReaction],
    agent_comment_refs: &[(String, String)],
) -> String {
    let mut out = String::new();
    out.push_str(&format!("<!-- repopilot-bot sha={sha} -->\n\n"));
    out.push_str("# Multi-Agent Review Summary\n\n");
    out.push_str(&format!("- Target: {target_url}\n"));
    out.push_str(&format!("- Head SHA: `{sha}`\n\n"));

    out.push_str("## Individual Agent Comments\n\n");
    if agent_comment_refs.is_empty() {
        out.push_str("- No individual agent comments were posted.\n\n");
    } else {
        for (name, id) in agent_comment_refs {
            out.push_str(&format!("- {}: comment id `{}`\n", name, id));
        }
        out.push('\n');
    }

    out.push_str("## Agent-to-Agent Reactions\n\n");
    if reactions.is_empty() {
        out.push_str("- Not enough agents to run cross-agent reactions.\n\n");
    } else {
        for reaction in reactions {
            out.push_str("---\n\n");
            out.push_str(&format!("### {} on Other Agents\n\n", reaction.provider_name));
            out.push_str(reaction.body.trim());
            out.push_str("\n\n");
        }
    }

    out
}

/// 동일 SHA/에이전트 코멘트를 식별하기 위한 마커 문자열을 만든다.
pub fn agent_marker(provider_id: &str, sha: &str) -> String {
    format!("<!-- repopilot-bot agent={} sha={} -->", provider_id, sha)
}
