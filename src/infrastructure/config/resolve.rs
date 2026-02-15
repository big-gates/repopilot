//! 설정 값(token/env/cmd 등)을 실제 런타임 값으로 해석하는 유틸리티.
//!
//! - 환경변수/프로세스 실행은 인프라 계층에서만 수행한다.

use std::env;
use std::process::Command;

use anyhow::{Context, Result};

use crate::application::config::{HostConfig, ProviderConfig};
use crate::application::ports::HostTokenResolution;

/// Provider(API key) 해석 결과.
#[derive(Debug, Clone)]
pub struct ProviderCredentialResolution {
    pub credential: Option<String>,
    pub source: Option<String>,
}

/// Host(VCS) 토큰을 해석한다.
pub fn resolve_host_token(host_cfg: Option<&HostConfig>) -> Result<HostTokenResolution> {
    let Some(cfg) = host_cfg else {
        return Ok(HostTokenResolution {
            token: None,
            source: None,
        });
    };

    if let Some(token) = cfg.token.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        return Ok(HostTokenResolution {
            token: Some(token.to_string()),
            source: Some("inline".to_string()),
        });
    }

    let mut env_hint: Option<String> = None;
    let mut cmd_hint: Option<String> = None;

    if let Some(env_name) = cfg.token_env.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        match env::var(env_name).ok().map(|v| v.trim().to_string()) {
            Some(v) if !v.is_empty() => {
                return Ok(HostTokenResolution {
                    token: Some(v),
                    source: Some(format!("env:{env_name}")),
                });
            }
            _ => {
                env_hint = Some(format!("env:{env_name} (missing)"));
            }
        }
    }

    if let Some(cmd) = cfg
        .token_command
        .as_ref()
        .filter(|v| !v.is_empty())
        .filter(|v| v.iter().any(|s| !s.trim().is_empty()))
    {
        let label = format!("cmd:{}", cmd.join(" "));
        match run_token_command(cmd) {
            Ok(token) => {
                let trimmed = token.trim();
                if !trimmed.is_empty() {
                    return Ok(HostTokenResolution {
                        token: Some(trimmed.to_string()),
                        source: Some(label),
                    });
                }
                cmd_hint = Some(format!("{label} (empty)"));
            }
            Err(_) => {
                cmd_hint = Some(format!("{label} (failed)"));
            }
        }
    }

    Ok(HostTokenResolution {
        token: None,
        source: cmd_hint.or(env_hint),
    })
}

/// Provider API key를 해석한다.
pub fn resolve_provider_api_key(cfg: &ProviderConfig) -> ProviderCredentialResolution {
    if let Some(key) = cfg.api_key.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        return ProviderCredentialResolution {
            credential: Some(key.to_string()),
            source: Some("inline".to_string()),
        };
    }

    let Some(env_name) = cfg
        .api_key_env
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    else {
        return ProviderCredentialResolution {
            credential: None,
            source: None,
        };
    };

    match env::var(env_name).ok().map(|v| v.trim().to_string()) {
        Some(v) if !v.is_empty() => ProviderCredentialResolution {
            credential: Some(v),
            source: Some(format!("env:{env_name}")),
        },
        _ => ProviderCredentialResolution {
            credential: None,
            source: Some(format!("env:{env_name} (missing)")),
        },
    }
}

pub fn provider_api_key_source_label(cfg: &ProviderConfig) -> Option<String> {
    if cfg.api_key.as_deref().map(str::trim).filter(|v| !v.is_empty()).is_some() {
        return Some("inline".to_string());
    }
    cfg.api_key_env
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|env_name| {
            if env::var(env_name)
                .ok()
                .filter(|v| !v.trim().is_empty())
                .is_some()
            {
                format!("env:{env_name}")
            } else {
                format!("env:{env_name} (missing)")
            }
        })
}

fn run_token_command(cmd: &[String]) -> Result<String> {
    let program = cmd
        .first()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .context("token_command is empty")?;
    let args: Vec<String> = cmd
        .iter()
        .skip(1)
        .map(|s| s.to_string())
        .collect();

    let output = Command::new(&program)
        .args(&args)
        .output()
        .with_context(|| format!("failed to run token command: {program}"))?;

    if !output.status.success() {
        anyhow::bail!("token command failed: {program} ({})", output.status);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
