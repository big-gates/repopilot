//! 리뷰 실행 컨텍스트(설정/대상/VCS 상태) 준비 단계.

use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::application::ports::VcsGateway;
use crate::application::usecases::review_pr::ReviewPrUseCase;
use crate::application::config::{Config, ProviderConfig};
use crate::domain::review::{ReviewComment, RunOptions};
use crate::domain::target::ReviewTarget;

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
        .context("failed to load repopilot config")?;

    let target = use_case
        .target_resolver
        .parse(&options.url)
        .context("failed to parse target URL")?;

    let host_cfg = config.host_config(target.host());
    let token_resolution = use_case
        .host_token_resolver
        .resolve(target.host(), host_cfg)
        .context("failed to resolve VCS host token")?;
    let token = token_resolution.token.clone();
    let token_resolved = token.is_some();

    render_status_dashboard(use_case, &config, &target, token_resolved, options.dry_run);
    if let Some(source) = token_resolution.source.as_deref() {
        use_case.reporter.kv("Host Token Source", source);
    }

    if !options.dry_run && token.is_none() {
        let host = target.host();
        let auth_hint = match &target {
            ReviewTarget::GitHub { .. } => format!("repopilot auth github --host {host}"),
            ReviewTarget::GitLab { .. } => format!("repopilot auth gitlab --host {host}"),
        };
        bail!(
            "missing VCS token for host '{}'. Configure hosts.{}.token / hosts.{}.token_env / hosts.{}.token_command (OAuth), run `{auth_hint}`, or use --dry-run",
            target.host(),
            target.host(),
            target.host(),
            target.host(),
        );
    }

    let vcs = use_case.vcs_factory.build(&target, host_cfg, token);

    use_case.reporter.section("Fetch Target");
    use_case.reporter.kv("Host", target.host());
    use_case.reporter.status("VCS", "fetching head SHA");
    let head_sha = match vcs.fetch_head_sha().await {
        Ok(sha) => {
            use_case.reporter.kv("Host Token Valid", "yes (API access ok)");
            sha
        }
        Err(err) => {
            if token_resolved {
                use_case
                    .reporter
                    .kv("Host Token Valid", "no (auth/permission check failed)");
            } else {
                use_case
                    .reporter
                    .kv("Host Token Valid", "no (token missing)");
            }
            return Err(err);
        }
    };
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

fn render_status_dashboard(
    use_case: &ReviewPrUseCase<'_>,
    config: &Config,
    target: &ReviewTarget,
    token_resolved: bool,
    dry_run: bool,
) {
    use_case.reporter.section("Status Dashboard");
    use_case.reporter.kv("Config", "ok");
    use_case.reporter.kv("Host", target.host());
    use_case.reporter.kv(
        "Host Token",
        if token_resolved {
            "resolved"
        } else if dry_run {
            "missing (dry-run allows continue)"
        } else {
            "missing"
        },
    );

    let guide_path = config
        .defaults
        .review_guide_path
        .clone()
        .unwrap_or_else(|| "not set".to_string());
    use_case.reporter.kv("Review Guide", &guide_path);
    let guide_status = if guide_path == "not set" {
        "not set"
    } else if Path::new(&guide_path).is_file() {
        "exists"
    } else {
        "missing"
    };
    use_case.reporter.kv("Guide Status", guide_status);

    use_case
        .reporter
        .kv("Comment Lang", config.comment_language().code());

    use_case.reporter.raw("Providers:");
    for line in provider_lines(config) {
        use_case.reporter.raw(&line);
    }
}

fn provider_lines(config: &Config) -> Vec<String> {
    vec![
        provider_line("openai", config.providers.openai.as_ref(), "codex"),
        provider_line("anthropic", config.providers.anthropic.as_ref(), "claude"),
        provider_line("gemini", config.providers.gemini.as_ref(), "gemini"),
    ]
}

fn provider_line(
    id: &str,
    cfg: Option<&ProviderConfig>,
    default_command: &str,
) -> String {
    let Some(cfg) = cfg else {
        return format!("  - {id:<10} not configured");
    };

    let enabled = cfg.is_enabled();
    let has_api_hint = cfg.api_key.is_some() || cfg.api_key_env.is_some();
    let state = if enabled { "enabled" } else { "disabled" };

    if has_api_hint {
        let model = cfg
            .model
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("default");
        return format!("  - {id:<10} {state:<8} mode=api model={model}");
    }

    let command = cfg
        .command
        .as_deref()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(default_command);
    let args = cfg.args.clone().unwrap_or_default().join(" ");
    if args.is_empty() {
        format!("  - {id:<10} {state:<8} mode=cli cmd={command}")
    } else {
        format!(
            "  - {id:<10} {state:<8} mode=cli cmd={} {}",
            command, args
        )
    }
}
