//! VCS 코멘트용 Markdown 렌더링 모듈.

use crate::domain::review::{AgentComment, AgentReaction, TokenUsage};

/// 리뷰 시작 상태를 나타내는 claim 코멘트 본문을 생성한다.
pub fn render_claim_markdown(sha: &str, target_url: &str) -> String {
    format!(
        "<!-- prpilot-bot claim sha={sha} -->\n\n# Multi-Agent Code Review\n\n- Target: {target_url}\n- Head SHA: `{sha}`\n\nReview in progress..."
    )
}

/// 에이전트별 개별 코멘트 본문을 생성한다.
pub fn render_agent_markdown(sha: &str, target_url: &str, agent: &AgentComment) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "<!-- prpilot-bot agent={} sha={} -->\n\n",
        agent.provider_id, sha
    ));
    out.push_str(&format!("# Agent Review: {}\n\n", agent.provider_name));
    out.push_str(&format!("- Target: {}\n", target_url));
    out.push_str(&format!("- Head SHA: `{}`\n", sha));
    out.push_str(&format!("- Token Usage: {}\n\n", format_usage(&agent.usage)));
    out.push_str(agent.body.trim());
    out.push('\n');
    out
}

/// 최종 요약 코멘트(상호 코멘트 + 사용량)를 생성한다.
pub fn render_final_summary_markdown(
    sha: &str,
    target_url: &str,
    reactions: &[AgentReaction],
    agent_comment_refs: &[(String, String)],
    usage_rows: &[(String, TokenUsage)],
) -> String {
    let mut out = String::new();
    out.push_str(&format!("<!-- prpilot-bot sha={sha} -->\n\n"));
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

    out.push_str("## Token Usage (Best Effort)\n\n");
    out.push_str("| Agent | Prompt | Completion | Total |\n");
    out.push_str("|---|---:|---:|---:|\n");
    for (name, usage) in usage_rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            name,
            opt_num(usage.prompt_tokens),
            opt_num(usage.completion_tokens),
            opt_num(usage.total_tokens)
        ));
    }

    out
}

/// 동일 SHA/에이전트 코멘트를 식별하기 위한 마커 문자열을 만든다.
pub fn agent_marker(provider_id: &str, sha: &str) -> String {
    format!("<!-- prpilot-bot agent={} sha={} -->", provider_id, sha)
}

/// 토큰 사용량을 콘솔/문서 표기용 문자열로 변환한다.
pub fn format_usage(usage: &TokenUsage) -> String {
    format!(
        "prompt={}, completion={}, total={}",
        opt_num(usage.prompt_tokens),
        opt_num(usage.completion_tokens),
        opt_num(usage.total_tokens)
    )
}

fn opt_num(value: Option<u64>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "n/a".to_string())
}
