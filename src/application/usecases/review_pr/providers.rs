//! Provider 실행(1차 리뷰/교차 반응) 단계.

use std::time::Instant;

use anyhow::{Context, Result, bail};
use futures::stream::{FuturesUnordered, StreamExt};

use crate::application::ports::ProviderAgent;
use crate::application::usecases::review_pr::{ReviewPrUseCase, context::ExecutionContext};
use crate::domain::policy::build_cross_agent_prompt;
use crate::domain::review::{AgentComment, AgentReaction, ProviderRun, ReviewRequest, TokenUsage};

/// 1차 리뷰 실행 결과 묶음.
pub(super) struct PrimaryReviewOutcome {
    pub primary_results: Vec<ProviderRun>,
    pub agent_comments: Vec<AgentComment>,
}

/// 리뷰 요청 객체를 구성한다(diff + system prompt).
pub(super) async fn build_review_request(
    use_case: &ReviewPrUseCase<'_>,
    ctx: &ExecutionContext,
) -> Result<ReviewRequest> {
    use_case.reporter.status("VCS", "fetching diff");
    let diff = ctx.vcs.fetch_diff().await?;
    use_case.reporter.kv("Diff Bytes", &diff.len().to_string());

    let max = ctx.config.max_diff_bytes();
    if diff.len() > max {
        let msg = format!(
            "warning: diff size ({} bytes) exceeds max_diff_bytes ({} bytes).",
            diff.len(),
            max
        );
        if !use_case.confirmer.confirm(&msg)? {
            bail!("cancelled by user");
        }
    }

    use_case.reporter.section("Prompt");
    let system_prompt = ctx
        .config
        .resolved_system_prompt()
        .context("failed to resolve system prompt with review guide")?;

    if let Some(path) = &ctx.config.defaults.review_guide_path {
        use_case.reporter.kv("Guide", path);
    } else {
        use_case.reporter.kv("Guide", "not set");
    }

    Ok(ReviewRequest {
        target_url: ctx.target.url().to_string(),
        head_sha: ctx.head_sha.clone(),
        diff,
        system_prompt,
        comment_language: ctx.config.comment_language(),
    })
}

/// 설정에서 활성 provider를 구성한다.
pub(super) fn build_enabled_providers(
    use_case: &ReviewPrUseCase<'_>,
    ctx: &ExecutionContext,
) -> Result<Vec<Box<dyn ProviderAgent>>> {
    let providers = use_case.provider_factory.build(&ctx.config);
    if providers.is_empty() {
        bail!(
            "no providers enabled. Configure providers.<name>.command (and optionally args/use_stdin), and ensure commands are installed"
        );
    }

    use_case.reporter.section("Providers (Primary Review)");
    use_case.reporter.kv("Enabled", &providers.len().to_string());
    Ok(providers)
}

/// provider 1차 리뷰를 병렬 실행한다.
pub(super) async fn run_primary_reviews(
    use_case: &ReviewPrUseCase<'_>,
    providers: &[Box<dyn ProviderAgent>],
    request: &ReviewRequest,
) -> PrimaryReviewOutcome {
    let mut primary_futures = FuturesUnordered::new();

    for provider in providers {
        let provider_id = provider.id().to_string();
        let provider_name = provider.name().to_string();
        use_case
            .reporter
            .provider_status(&provider_name, "running", None);
        let provider_request = request.clone();
        primary_futures.push(async move {
            let started = Instant::now();
            match provider.review(&provider_request).await {
                Ok(resp) => {
                    let display_name = provider_name.clone();
                    (
                        display_name,
                        ProviderRun {
                            id: provider_id,
                            name: provider_name,
                            body: resp.content,
                            usage: resp.usage,
                        },
                        false,
                        started.elapsed().as_secs_f32(),
                    )
                }
                Err(err) => {
                    let display_name = provider_name.clone();
                    (
                        display_name,
                        ProviderRun {
                            id: provider_id,
                            name: provider_name,
                            body: format!("_Error: {}_", err),
                            usage: TokenUsage::default(),
                        },
                        true,
                        started.elapsed().as_secs_f32(),
                    )
                }
            }
        });
    }

    let mut primary_results = Vec::new();
    while let Some((name, run, is_error, sec)) = primary_futures.next().await {
        if is_error {
            use_case
                .reporter
                .provider_status(&name, "error", Some(&format!("{sec:.1}s")));
        } else {
            use_case
                .reporter
                .provider_status(&name, "done", Some(&format!("{sec:.1}s")));
        }
        primary_results.push(run);
    }

    let agent_comments: Vec<AgentComment> = primary_results
        .iter()
        .map(|r| AgentComment {
            provider_id: r.id.clone(),
            provider_name: r.name.clone(),
            body: r.body.clone(),
            usage: r.usage.clone(),
        })
        .collect();

    PrimaryReviewOutcome {
        primary_results,
        agent_comments,
    }
}

/// provider 간 상호 코멘트를 병렬 실행한다.
pub(super) async fn run_cross_agent_reactions(
    use_case: &ReviewPrUseCase<'_>,
    providers: &[Box<dyn ProviderAgent>],
    request: &ReviewRequest,
    primary_results: &[ProviderRun],
) -> Vec<AgentReaction> {
    if providers.len() <= 1 {
        return Vec::new();
    }

    use_case.reporter.section("Providers (Cross-Agent Reactions)");

    let mut reaction_futures = FuturesUnordered::new();

    for provider in providers {
        let provider_name = provider.name().to_string();
        use_case
            .reporter
            .provider_status(&provider_name, "running", None);
        let prompt = build_cross_agent_prompt(
            &request.target_url,
            &request.head_sha,
            provider.id(),
            &provider_name,
            request.comment_language,
            primary_results,
        );

        reaction_futures.push(async move {
            let started = Instant::now();
            match provider.review_prompt(&prompt).await {
                Ok(resp) => {
                    let display_name = provider_name.clone();
                    (
                        display_name,
                        AgentReaction {
                            provider_name,
                            body: resp.content,
                        },
                        false,
                        started.elapsed().as_secs_f32(),
                    )
                }
                Err(err) => {
                    let display_name = provider_name.clone();
                    (
                        display_name,
                        AgentReaction {
                            provider_name,
                            body: format!("_Error: {}_", err),
                        },
                        true,
                        started.elapsed().as_secs_f32(),
                    )
                }
            }
        });
    }

    let mut reactions = Vec::new();
    while let Some((name, reaction, is_error, sec)) = reaction_futures.next().await {
        if is_error {
            use_case
                .reporter
                .provider_status(&name, "error", Some(&format!("{sec:.1}s")));
        } else {
            use_case
                .reporter
                .provider_status(&name, "done", Some(&format!("{sec:.1}s")));
        }
        reactions.push(reaction);
    }

    reactions
}
