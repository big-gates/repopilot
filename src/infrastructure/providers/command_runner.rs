//! Provider CLI 실행기.

use std::io::IsTerminal;
use std::process::Stdio;

use anyhow::{Context, Result, bail};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::domain::review::ProviderResponse;
use crate::infrastructure::config::command_exists;
use crate::infrastructure::config::ProviderCommandSpec;

use super::usage_parser::parse_usage;

/// provider 명령을 실행하고, 필요 시 stdin 비터미널 오류를 자동 재시도한다.
pub async fn run_provider_command(
    provider_name: &str,
    spec: &ProviderCommandSpec,
    prompt: &str,
    auth_command: Option<&[String]>,
    auto_auth: bool,
) -> Result<ProviderResponse> {
    let mut current = spec.clone();
    let mut tried_stdin_fallback = false;
    let mut tried_auth = false;

    loop {
        match run_provider_command_once(provider_name, &current, prompt).await {
            Ok(text) => return Ok(text),
            Err(err) => {
                let msg = format!("{err:#}");
                let lower = msg.to_lowercase();

                // Some CLIs reject piped stdin and require argument-based input.
                if current.use_stdin
                    && !tried_stdin_fallback
                    && lower.contains("stdin is not a terminal")
                {
                    tried_stdin_fallback = true;
                    current.use_stdin = false;
                    continue;
                }

                // OAuth login retry (interactive only).
                let interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
                if auto_auth
                    && interactive
                    && !tried_auth
                    && seems_auth_failure(&lower)
                    && let Some(cmd) = auth_command
                {
                    tried_auth = true;
                    eprintln!("{provider_name}: authentication required; running OAuth login...");
                    run_auth_command(provider_name, cmd).await?;
                    continue;
                }

                return Err(err);
            }
        }
    }
}

fn seems_auth_failure(lower_msg: &str) -> bool {
    // Keep this heuristic conservative to avoid running interactive login on unrelated failures.
    lower_msg.contains("unauthorized")
        || lower_msg.contains("not authenticated")
        || lower_msg.contains("authentication required")
        || lower_msg.contains("not logged in")
        || lower_msg.contains("please login")
        || lower_msg.contains("please log in")
        || (lower_msg.contains("run") && lower_msg.contains("login"))
        || lower_msg.contains("sign in")
        || lower_msg.contains("login required")
        || lower_msg.contains("oauth")
}

async fn run_auth_command(provider_name: &str, cmd: &[String]) -> Result<()> {
    let program = cmd
        .first()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .context("auth_command is empty")?;
    let args: Vec<&str> = cmd.iter().skip(1).map(|s| s.as_str()).collect();

    if !command_exists(program) {
        bail!("auth command program not found in PATH: '{program}'");
    }

    if args.is_empty() && program == "claude" {
        eprintln!("{provider_name}: Claude login is interactive. Type `/login`, finish auth, then exit.");
    } else if args.is_empty() && program == "gemini" {
        eprintln!("{provider_name}: Gemini login is interactive. Choose Login with Google, finish auth, then exit.");
    }

    let status = Command::new(program)
        .args(&args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .with_context(|| format!("failed to run auth command for {}", provider_name))?;

    if !status.success() {
        bail!("auth command exited with {}", status);
    }
    Ok(())
}

async fn run_provider_command_once(
    provider_name: &str,
    spec: &ProviderCommandSpec,
    prompt: &str,
) -> Result<ProviderResponse> {
    // {prompt} 치환 또는 stdin 전달 규칙에 따라 최종 실행 인자를 구성한다.
    let mut args = Vec::new();
    let mut prompt_in_args = false;
    for arg in &spec.args {
        if arg.contains("{prompt}") {
            prompt_in_args = true;
            args.push(arg.replace("{prompt}", prompt));
        } else {
            args.push(arg.clone());
        }
    }

    if !spec.use_stdin && !prompt_in_args {
        args.push(prompt.to_string());
    }

    let mut cmd = Command::new(&spec.command);
    cmd.args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if spec.use_stdin {
        cmd.stdin(Stdio::piped());
    }

    let mut child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn {} command '{}'", provider_name, spec.command))?;

    if spec.use_stdin {
        let mut stdin = child
            .stdin
            .take()
            .context("failed to open provider command stdin")?;
        stdin
            .write_all(prompt.as_bytes())
            .await
            .context("failed to write prompt to provider command stdin")?;
        drop(stdin);
    }

    let output = child
        .wait_with_output()
        .await
        .context("provider command execution failed")?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    let usage = parse_usage(&stdout, &stderr);

    if !output.status.success() {
        bail!(
            "{} command failed ({}): {}",
            provider_name,
            output.status,
            if stderr.is_empty() {
                "no stderr output"
            } else {
                stderr.as_str()
            }
        );
    }

    if stdout.is_empty() {
        if stderr.is_empty() {
            bail!("{} command returned empty output", provider_name);
        }
        return Ok(ProviderResponse {
            content: stderr,
            usage,
        });
    }

    Ok(ProviderResponse {
        content: stdout,
        usage,
    })
}
