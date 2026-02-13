//! SHA 기반 중복 방지와 claim 코멘트 처리 단계.

use anyhow::Result;

use crate::application::usecases::review_pr::{ReviewPrUseCase, context::ExecutionContext};
use crate::domain::policy::{
    find_comment_with_marker, markers_for_sha, upsert_comment_cache,
};
use crate::domain::review::RunOptions;

/// claim 단계의 판단 결과.
pub(super) enum ClaimDecision {
    Skip,
    Continue { claim_comment_id: Option<String> },
}

/// 기존 final/claim 마커를 검사하고, 필요 시 claim 코멘트를 생성/업데이트한다.
pub(super) async fn prepare_claim_comment(
    use_case: &ReviewPrUseCase<'_>,
    options: &RunOptions,
    ctx: &mut ExecutionContext,
) -> Result<ClaimDecision> {
    if options.dry_run {
        return Ok(ClaimDecision::Continue {
            claim_comment_id: None,
        });
    }

    let markers = markers_for_sha(&ctx.head_sha);
    let final_comment = find_comment_with_marker(&ctx.existing_comments, &markers.final_marker);
    let claim_comment = find_comment_with_marker(&ctx.existing_comments, &markers.claim_marker);

    if !options.force && (final_comment.is_some() || claim_comment.is_some()) {
        use_case
            .reporter
            .status("Dedup", "already claimed/reviewed for current SHA; skipping");
        return Ok(ClaimDecision::Skip);
    }

    let chosen_comment_id = claim_comment
        .or(if options.force { final_comment } else { None })
        .map(|c| c.id.clone());

    let claim_markdown = use_case
        .renderer
        .render_claim(&ctx.head_sha, ctx.target.url());

    if let Some(comment_id) = chosen_comment_id {
        let updated = ctx.vcs.update_comment(&comment_id, &claim_markdown).await?;
        upsert_comment_cache(&mut ctx.existing_comments, updated);
        use_case
            .reporter
            .status("Claim", "updated existing claim comment");
        Ok(ClaimDecision::Continue {
            claim_comment_id: Some(comment_id),
        })
    } else {
        let created = ctx.vcs.create_comment(&claim_markdown).await?;
        let id = created.id.clone();
        upsert_comment_cache(&mut ctx.existing_comments, created);
        use_case.reporter.status("Claim", "created claim comment");
        Ok(ClaimDecision::Continue {
            claim_comment_id: Some(id),
        })
    }
}
