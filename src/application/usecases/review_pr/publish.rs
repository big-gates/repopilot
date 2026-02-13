//! 개별/최종 코멘트 렌더링 및 게시 단계.

use anyhow::{Context, Result};

use crate::application::usecases::review_pr::{ReviewPrUseCase, context::ExecutionContext};
use crate::domain::policy::{agent_marker, find_comment_with_marker, upsert_comment_cache};
use crate::domain::review::{AgentComment, AgentReaction, RunOptions, TokenUsage};

/// 개별 에이전트 코멘트를 출력(dry-run) 또는 게시(upsert)한다.
pub(super) async fn publish_agent_comments(
    use_case: &ReviewPrUseCase<'_>,
    options: &RunOptions,
    ctx: &mut ExecutionContext,
    agent_comments: &[AgentComment],
) -> Result<Vec<(String, String)>> {
    let mut agent_comment_refs: Vec<(String, String)> = Vec::new();

    if options.dry_run {
        use_case.reporter.section("Dry Run: Individual Comments");
        for agent in agent_comments {
            use_case
                .reporter
                .raw(&format!("--- {} ---", agent.provider_name));
            let markdown = use_case
                .renderer
                .render_agent(&ctx.head_sha, ctx.target.url(), agent);
            use_case.reporter.raw(&markdown);
        }
        return Ok(agent_comment_refs);
    }

    use_case.reporter.section("Post Individual Comments");
    for agent in agent_comments {
        let marker = agent_marker(&agent.provider_id, &ctx.head_sha);
        let markdown = use_case
            .renderer
            .render_agent(&ctx.head_sha, ctx.target.url(), agent);
        let existing = find_comment_with_marker(&ctx.existing_comments, &marker).map(|c| c.id.clone());

        let posted = if let Some(comment_id) = existing {
            use_case
                .reporter
                .status(&agent.provider_name, "updating comment");
            ctx.vcs.update_comment(&comment_id, &markdown).await?
        } else {
            use_case
                .reporter
                .status(&agent.provider_name, "creating comment");
            ctx.vcs.create_comment(&markdown).await?
        };

        agent_comment_refs.push((agent.provider_name.clone(), posted.id.clone()));
        upsert_comment_cache(&mut ctx.existing_comments, posted);
    }

    Ok(agent_comment_refs)
}

/// 최종 요약 코멘트를 출력(dry-run) 또는 claim 코멘트를 갱신한다.
pub(super) async fn publish_final_summary(
    use_case: &ReviewPrUseCase<'_>,
    options: &RunOptions,
    ctx: &mut ExecutionContext,
    claim_comment_id: Option<&str>,
    reactions: &[AgentReaction],
    agent_comment_refs: &[(String, String)],
    usage_rows: &[(String, TokenUsage)],
) -> Result<()> {
    let final_markdown = use_case.renderer.render_final(
        &ctx.head_sha,
        ctx.target.url(),
        reactions,
        agent_comment_refs,
        usage_rows,
    );

    if options.dry_run {
        use_case.reporter.section("Dry Run: Final Summary Comment");
        use_case.reporter.raw(&final_markdown);
        return Ok(());
    }

    let claim_comment_id = claim_comment_id
        .context("internal error: missing claim comment id for non-dry-run")?;

    ctx.vcs.update_comment(claim_comment_id, &final_markdown).await?;
    use_case.reporter.section("Done");
    use_case.reporter.status("VCS", "final summary comment posted");
    Ok(())
}
