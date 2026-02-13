//! 리뷰 실행 컨텍스트(설정/대상/VCS 상태) 준비 단계.

use anyhow::{Context, Result, bail};

use crate::application::ports::VcsGateway;
use crate::application::usecases::review_pr::ReviewPrUseCase;
use crate::domain::review::{ReviewComment, RunOptions};
use crate::domain::target::ReviewTarget;
use crate::infrastructure::config::Config;

/// 리뷰 유스케이스 전 구간에서 공유되는 실행 상태.
pub(super) struct ExecutionContext {
    pub config: Config,
    pub target: ReviewTarget,
    pub vcs: Box<dyn VcsGateway>,
    pub head_sha: String,
    pub existing_comments: Vec<ReviewComment>,
}

/// 설정 로딩, 대상 파싱, VCS 인증/HEAD SHA 조회까지 선행한다.
pub(super) async fn load_execution_context(
    use_case: &ReviewPrUseCase<'_>,
    options: &RunOptions,
) -> Result<ExecutionContext> {
    use_case.reporter.section("Load Config");
    let config = use_case
        .config_repo
        .load()
        .context("failed to load prpilot config")?;

    let target = use_case
        .target_resolver
        .parse(&options.url)
        .context("failed to parse target URL")?;

    let host_cfg = config.host_config(target.host());
    let token = host_cfg.and_then(|h| h.resolve_token());

    if !options.dry_run && token.is_none() {
        bail!(
            "missing VCS token for host '{}'. Configure hosts.{}.token or hosts.{}.token_env in config, or use --dry-run",
            target.host(),
            target.host(),
            target.host(),
        );
    }

    let vcs = use_case.vcs_factory.build(&target, host_cfg, token);

    use_case.reporter.section("Fetch Target");
    use_case.reporter.kv("Host", target.host());
    use_case.reporter.status("VCS", "fetching head SHA");
    let head_sha = vcs.fetch_head_sha().await?;
    use_case.reporter.kv("Head SHA", &head_sha);

    let existing_comments = if options.dry_run {
        Vec::new()
    } else {
        vcs.list_comments().await?
    };

    Ok(ExecutionContext {
        config,
        target,
        vcs,
        head_sha,
        existing_comments,
    })
}
