//! 최신 버전 확인 유스케이스.

use anyhow::Result;
use url::Url;

use crate::application::ports::{ConfigRepository, HostTokenResolver, UpdateChecker};
use crate::application::config::Config;

/// 업데이트 안내 메시지 생성용 데이터.
#[derive(Debug, Clone)]
pub struct UpdateNotice {
    pub current_version: String,
    pub latest_version: String,
    pub download_url: Option<String>,
}

/// 설정 기반 원격 최신 버전을 조회하고 업데이트 필요 여부를 판단한다.
pub struct CheckUpdateUseCase<'a> {
    pub config_repo: &'a dyn ConfigRepository,
    pub host_token_resolver: &'a dyn HostTokenResolver,
    pub update_checker: &'a dyn UpdateChecker,
}

impl<'a> CheckUpdateUseCase<'a> {
    /// 최신 버전이 있을 때만 안내 정보를 반환한다.
    /// - 설정/네트워크 오류는 사용자 실행 흐름을 막지 않기 위해 조용히 무시한다.
    pub async fn execute(&self) -> Result<Option<UpdateNotice>> {
        let config = match self.config_repo.load() {
            Ok(cfg) => cfg,
            Err(_) => return Ok(None),
        };

        let Some(check_url) = config.defaults.update_check_url.as_deref() else {
            return Ok(None);
        };

        let timeout_ms = config.defaults.update_timeout_ms.unwrap_or(1200);
        let host_token = resolve_host_token(&config, check_url, self.host_token_resolver);
        let Some(latest) = self
            .update_checker
            .fetch_latest(check_url, host_token.as_deref(), timeout_ms)
            .await?
        else {
            return Ok(None);
        };

        let current = env!("CARGO_PKG_VERSION");
        if !is_newer_version(current, &latest.version) {
            return Ok(None);
        }

        let download_url = latest
            .download_url
            .or_else(|| config.defaults.update_download_url.clone());

        Ok(Some(UpdateNotice {
            current_version: current.to_string(),
            latest_version: latest.version,
            download_url,
        }))
    }
}

fn resolve_host_token(
    config: &Config,
    raw_url: &str,
    resolver: &dyn HostTokenResolver,
) -> Option<String> {
    let parsed = Url::parse(raw_url).ok()?;
    let host = parsed.host_str()?;
    let host_cfg = config.host_config(host);
    resolver.resolve(host, host_cfg).ok()?.token
}

fn is_newer_version(current: &str, latest: &str) -> bool {
    let Some(current_parts) = parse_version_parts(current) else {
        return false;
    };
    let Some(latest_parts) = parse_version_parts(latest) else {
        return false;
    };

    let len = current_parts.len().max(latest_parts.len());
    for idx in 0..len {
        let left = *current_parts.get(idx).unwrap_or(&0);
        let right = *latest_parts.get(idx).unwrap_or(&0);
        if right > left {
            return true;
        }
        if right < left {
            return false;
        }
    }

    false
}

fn parse_version_parts(raw: &str) -> Option<Vec<u64>> {
    let s = raw.trim().trim_start_matches('v');
    let start = s.find(|c: char| c.is_ascii_digit())?;
    let mut normalized = String::new();
    for ch in s[start..].chars() {
        if ch.is_ascii_digit() || ch == '.' {
            normalized.push(ch);
        } else {
            break;
        }
    }

    if normalized.is_empty() {
        return None;
    }

    let mut out = Vec::new();
    for part in normalized.split('.') {
        if part.is_empty() {
            continue;
        }
        let Ok(v) = part.parse::<u64>() else {
            return None;
        };
        out.push(v);
    }

    if out.is_empty() { None } else { Some(out) }
}
