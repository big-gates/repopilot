//! Provider OAuth 인증 포트 구현(codex/claude/gemini).

use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::application::config::ProviderConfig;
use crate::application::ports::{ProviderAuthKind, ProviderAuthenticator};
use crate::infrastructure::config::command_exists;

/// provider CLI를 통해 OAuth 로그인을 수행한다.
pub struct ProviderAuthenticatorAdapter;

impl ProviderAuthenticator for ProviderAuthenticatorAdapter {
    fn authenticate(
        &self,
        kind: ProviderAuthKind,
        provider_cfg: Option<&ProviderConfig>,
    ) -> Result<()> {
        let cmd = auth_command(kind, provider_cfg);
        run_interactive(kind, &cmd)
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

    match kind {
        ProviderAuthKind::Codex => vec![program, "login".to_string()],
        ProviderAuthKind::Claude => vec![program, "auth".to_string(), "login".to_string()],
        // Gemini CLI는 interactive 세션에서 로그인 플로우를 진행한다.
        ProviderAuthKind::Gemini => vec![program],
    }
}

fn default_program(kind: ProviderAuthKind) -> &'static str {
    match kind {
        ProviderAuthKind::Codex => "codex",
        ProviderAuthKind::Claude => "claude",
        ProviderAuthKind::Gemini => "gemini",
    }
}

fn run_interactive(kind: ProviderAuthKind, cmd: &[String]) -> Result<()> {
    let program = cmd
        .first()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .context("auth command is empty")?;
    let args: Vec<&str> = cmd.iter().skip(1).map(|s| s.as_str()).collect();

    if !command_exists(program) {
        bail!(
            "provider CLI not found in PATH: '{program}'.\n\
Install the CLI or use API mode instead.\n\
- Codex: npm install -g @openai/codex (or set OPENAI_API_KEY)\n\
- Claude Code: npm install -g @anthropic-ai/claude-code (or set ANTHROPIC_API_KEY)\n\
- Gemini CLI: npm install -g @google/gemini-cli (or set GEMINI_API_KEY)"
        );
    }

    match kind {
        ProviderAuthKind::Claude if args.is_empty() => {
            eprintln!("Claude: start the CLI, then type `/login` to authenticate, then exit.");
        }
        ProviderAuthKind::Gemini if args.is_empty() => {
            eprintln!("Gemini: start the CLI, choose Login with Google, then exit when done.");
        }
        _ => {}
    }

    let status = Command::new(program)
        .args(&args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to run auth command: {} {}", program, args.join(" ")))?;

    if !status.success() {
        bail!("auth command exited with {status}");
    }
    Ok(())
}
