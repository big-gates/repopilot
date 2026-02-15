//! Provider OAuth 인증 포트 구현(codex/claude/gemini).

use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::application::config::ProviderConfig;
use crate::application::ports::{ProviderAuthKind, ProviderAuthenticator};

/// provider CLI를 통해 OAuth 로그인을 수행한다.
pub struct ProviderAuthenticatorAdapter;

impl ProviderAuthenticator for ProviderAuthenticatorAdapter {
    fn authenticate(
        &self,
        kind: ProviderAuthKind,
        provider_cfg: Option<&ProviderConfig>,
    ) -> Result<()> {
        let cmd = auth_command(kind, provider_cfg);
        run_interactive(&cmd)
    }
}

fn auth_command(kind: ProviderAuthKind, provider_cfg: Option<&ProviderConfig>) -> Vec<String> {
    if let Some(cmd) = provider_cfg
        .and_then(|cfg| cfg.auth_command.clone())
        .filter(|v| !v.is_empty())
    {
        return cmd;
    }

    let program = provider_cfg
        .and_then(|cfg| cfg.command.clone())
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default_program(kind).to_string());

    vec![program, "login".to_string()]
}

fn default_program(kind: ProviderAuthKind) -> &'static str {
    match kind {
        ProviderAuthKind::Codex => "codex",
        ProviderAuthKind::Claude => "claude",
        ProviderAuthKind::Gemini => "gemini",
    }
}

fn run_interactive(cmd: &[String]) -> Result<()> {
    let program = cmd
        .first()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .context("auth command is empty")?;
    let args: Vec<&str> = cmd.iter().skip(1).map(|s| s.as_str()).collect();

    let status = Command::new(program)
        .args(&args)
        .status()
        .with_context(|| format!("failed to run auth command: {} {}", program, args.join(" ")))?;

    if !status.success() {
        bail!("auth command exited with {status}");
    }
    Ok(())
}

