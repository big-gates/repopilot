//! 도메인 정책(중복 방지 규칙, 프롬프트 구성, 집계 규칙).

use crate::domain::review::{
    CommentLanguage, ProviderRun, ReviewComment, ReviewMarkers, TokenUsage, UsageTotals,
};

pub fn markers_for_sha(sha: &str) -> ReviewMarkers {
    ReviewMarkers {
        final_marker: format!("<!-- repopilot-bot sha={} -->", sha),
        claim_marker: format!("<!-- repopilot-bot claim sha={} -->", sha),
    }
}

pub fn agent_marker(provider_id: &str, sha: &str) -> String {
    format!("<!-- repopilot-bot agent={} sha={} -->", provider_id, sha)
}

pub fn find_comment_with_marker<'a>(
    comments: &'a [ReviewComment],
    marker: &str,
) -> Option<&'a ReviewComment> {
    comments.iter().find(|c| c.body.contains(marker))
}

pub fn upsert_comment_cache(comments: &mut Vec<ReviewComment>, comment: ReviewComment) {
    if let Some(idx) = comments.iter().position(|c| c.id == comment.id) {
        comments[idx] = comment;
    } else {
        comments.push(comment);
    }
}

pub fn add_usage_total(
    usage_totals: &mut UsageTotals,
    provider_id: &str,
    provider_name: &str,
    usage: &TokenUsage,
) {
    let entry = usage_totals
        .entry(provider_id.to_string())
        .or_insert_with(|| (provider_name.to_string(), TokenUsage::default()));
    entry.1.add_from(usage);
}

pub fn build_cross_agent_prompt(
    target_url: &str,
    head_sha: &str,
    self_id: &str,
    self_name: &str,
    comment_language: CommentLanguage,
    primary_results: &[ProviderRun],
) -> String {
    let mut out = String::new();
    out.push_str("You are participating in a multi-agent code review.\n");
    out.push_str("Analyze other agents' findings and provide your perspective.\n");
    out.push_str("Output language requirement:\n");
    out.push_str(comment_language.prompt_instruction());
    out.push_str("\n\n");
    out.push_str(&format!("Target URL: {}\n", target_url));
    out.push_str(&format!("Head SHA: {}\n\n", head_sha));
    out.push_str("Other agents' findings:\n\n");

    for result in primary_results {
        if result.id == self_id {
            continue;
        }
        out.push_str(&format!("## {}\n", result.name));
        out.push_str(result.body.trim());
        out.push_str("\n\n");
    }

    out.push_str(&format!(
        "Now write {}'s reaction to other agents.\n",
        self_name
    ));
    out.push_str(
        "Use Markdown sections in this order: Agreements, Disagreements, Missed Risks, Suggested Resolution.\n",
    );
    out
}
