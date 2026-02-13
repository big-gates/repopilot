//! 최신 버전 조회 포트 구현 어댑터.

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;

use crate::application::ports::{LatestVersionInfo, UpdateChecker};

/// HTTP endpoint에서 최신 버전을 조회하는 어댑터.
pub struct HttpUpdateChecker;

#[async_trait]
impl UpdateChecker for HttpUpdateChecker {
    async fn fetch_latest(
        &self,
        url: &str,
        token: Option<&str>,
        timeout_ms: u64,
    ) -> Result<Option<LatestVersionInfo>> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()?;

        let mut req = client.get(url);
        if let Some(token) = token {
            req = req
                .header("PRIVATE-TOKEN", token)
                .header("Authorization", format!("Bearer {token}"));
        }

        let resp = match req.send().await {
            Ok(resp) => resp,
            Err(_) => return Ok(None),
        };

        if !resp.status().is_success() {
            return Ok(None);
        }

        let content_type = resp
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let body = match resp.text().await {
            Ok(text) => text,
            Err(_) => return Ok(None),
        };

        if content_type.contains("json") || body.trim_start().starts_with('{') {
            return Ok(parse_json_payload(&body));
        }

        Ok(parse_plain_payload(&body))
    }
}

fn parse_plain_payload(raw: &str) -> Option<LatestVersionInfo> {
    let version = raw.trim();
    if version.is_empty() {
        return None;
    }

    Some(LatestVersionInfo {
        version: version.to_string(),
        download_url: None,
    })
}

fn parse_json_payload(raw: &str) -> Option<LatestVersionInfo> {
    let json: Value = serde_json::from_str(raw).ok()?;
    let version = find_version(&json)?;
    let download_url = find_download_url(&json);

    Some(LatestVersionInfo {
        version,
        download_url,
    })
}

fn find_version(json: &Value) -> Option<String> {
    // GitLab latest release API: tag_name
    str_at(json, &["tag_name"])
        .or_else(|| str_at(json, &["latest_version"]))
        .or_else(|| str_at(json, &["version"]))
        .or_else(|| str_at(json, &["tag"]))
        .or_else(|| str_at(json, &["name"]))
}

fn find_download_url(json: &Value) -> Option<String> {
    str_at(json, &["download_url"])
        .or_else(|| str_at(json, &["url"]))
        .or_else(|| str_at(json, &["assets", "links", "0", "url"]))
        .or_else(|| str_at(json, &["assets", "sources", "0", "url"]))
}

fn str_at(value: &Value, path: &[&str]) -> Option<String> {
    let mut cur = value;
    for key in path {
        if let Ok(idx) = key.parse::<usize>() {
            cur = cur.get(idx)?;
        } else {
            cur = cur.get(*key)?;
        }
    }

    cur.as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}
