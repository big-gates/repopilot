//! PR/MR 리뷰 실행의 전체 오케스트레이션 유스케이스.

mod context;
mod dedupe;
mod providers;
mod publish;

use anyhow::Result;

use crate::application::ports::{
    ConfigRepository, HostTokenResolver, MarkdownRenderer, ProviderFactory, Reporter,
    SystemPromptResolver, TargetResolver, UserConfirmer, VcsFactory,
};
use crate::domain::review::RunOptions;

use context::load_execution_context;
use dedupe::{ClaimDecision, prepare_claim_comment};
use providers::{
    build_enabled_providers, build_review_request, run_cross_agent_reactions, run_primary_reviews,
};
use publish::{publish_agent_comments, publish_final_summary};

/// URL 입력부터 VCS/제공자 호출, 코멘트 업서트까지 전체 흐름을 조율한다.
pub struct ReviewPrUseCase<'a> {
    pub config_repo: &'a dyn ConfigRepository,
    pub host_token_resolver: &'a dyn HostTokenResolver,
    pub system_prompt_resolver: &'a dyn SystemPromptResolver,
    pub target_resolver: &'a dyn TargetResolver,
    pub vcs_factory: &'a dyn VcsFactory,
    pub provider_factory: &'a dyn ProviderFactory,
    pub renderer: &'a dyn MarkdownRenderer,
    pub reporter: &'a dyn Reporter,
    pub confirmer: &'a dyn UserConfirmer,
}

impl<'a> ReviewPrUseCase<'a> {
    /// 리뷰 본 실행 진입점.
    /// dry-run/force 옵션을 반영해 중복 방지, 코멘트 게시, 최종 요약 게시를 수행한다.
    pub async fn execute(&self, options: RunOptions) -> Result<()> {
        self.reporter.section("Session");
        self.reporter.kv("Target", &options.url);
        self.reporter.kv(
            "Mode",
            if options.dry_run {
                "dry-run"
            } else {
                "post-comment"
            },
        );
        if options.force {
            self.reporter.kv("Force", "enabled");
        }

        let mut ctx = load_execution_context(self, &options).await?;

        let claim_comment_id = match prepare_claim_comment(self, &options, &mut ctx).await? {
            ClaimDecision::Skip => return Ok(()),
            ClaimDecision::Continue { claim_comment_id } => claim_comment_id,
        };

        let request = build_review_request(self, &ctx).await?;
        let providers = build_enabled_providers(self, &ctx)?;
        let primary_outcome = run_primary_reviews(self, &providers, &request).await;

        let agent_comment_refs =
            publish_agent_comments(self, &options, &mut ctx, &primary_outcome.agent_comments)
                .await?;

        let reactions = run_cross_agent_reactions(
            self,
            &providers,
            &request,
            &primary_outcome.primary_results,
        )
        .await;

        publish_final_summary(
            self,
            &options,
            &mut ctx,
            claim_comment_id.as_deref(),
            &reactions,
            &agent_comment_refs,
        )
        .await?;

        Ok(())
    }
}
