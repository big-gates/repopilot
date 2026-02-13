//! Provider CLI 실행기.

use std::process::Stdio;

use anyhow::{Context, Result, bail};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::domain::review::ProviderResponse;
use crate::infrastructure::config::ProviderCommandSpec;

use super::usage_parser::parse_usage;

/// provider 명령을 실행하고, 필요 시 stdin 비터미널 오류를 자동 재시도한다.
pub async fn run_provider_command(
    provider_name: &str,
    spec: &ProviderCommandSpec,
    prompt: &str,
) -> Result<ProviderResponse> {
    match run_provider_command_once(provider_name, spec, prompt).await {
        Ok(text) => Ok(text),
        Err(err) => {
            // Some CLIs reject piped stdin and require argument-based input.
            let msg = format!("{err:#}");
            if spec.use_stdin && msg.contains("stdin is not a terminal") {
                let mut fallback = spec.clone();
                fallback.use_stdin = false;
                return run_provider_command_once(provider_name, &fallback, prompt)
                    .await
                    .with_context(|| {
                        format!(
                            "{}: stdin mode failed; retried without stdin but still failed",
                            provider_name
                        )
                    });
            }
            Err(err)
        }
    }
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
