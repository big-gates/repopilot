//! 입력 URL을 GitHub PR / GitLab MR 대상으로 해석하는 모듈.

use anyhow::{Result, bail};
use url::Url;

#[derive(Debug, Clone)]
pub enum ReviewTarget {
    GitHub {
        host: String,
        owner: String,
        repo: String,
        number: u64,
        url: String,
    },
    GitLab {
        host: String,
        project_path: String,
        iid: u64,
        url: String,
    },
}

impl ReviewTarget {
    /// URL 패턴을 보고 GitHub/GitLab 대상을 자동 감지한다.
    pub fn parse(input: &str) -> Result<Self> {
        let url = Url::parse(input)?;
        let host = url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("URL host is missing"))?
            .to_string();

        let segments: Vec<String> = url
            .path_segments()
            .map(|s| s.filter(|p| !p.is_empty()).map(ToString::to_string).collect())
            .unwrap_or_default();

        if let Some(target) = parse_github(&host, &segments, input) {
            return Ok(target);
        }

        if let Some(target) = parse_gitlab(&host, &segments, input) {
            return Ok(target);
        }

        bail!("unsupported URL format: {input}")
    }

    pub fn host(&self) -> &str {
        match self {
            ReviewTarget::GitHub { host, .. } => host,
            ReviewTarget::GitLab { host, .. } => host,
        }
    }

    pub fn url(&self) -> &str {
        match self {
            ReviewTarget::GitHub { url, .. } => url,
            ReviewTarget::GitLab { url, .. } => url,
        }
    }
}

fn parse_github(host: &str, segments: &[String], input: &str) -> Option<ReviewTarget> {
    // /owner/repo/pull/<number>
    if segments.len() < 4 {
        return None;
    }
    if segments[2] != "pull" {
        return None;
    }

    let number = segments[3].parse().ok()?;

    Some(ReviewTarget::GitHub {
        host: host.to_string(),
        owner: segments[0].clone(),
        repo: segments[1].clone(),
        number,
        url: input.to_string(),
    })
}

fn parse_gitlab(host: &str, segments: &[String], input: &str) -> Option<ReviewTarget> {
    // /group/.../project/-/merge_requests/<iid>
    let sep = segments.iter().position(|s| s == "-")?;
    if sep + 2 >= segments.len() {
        return None;
    }
    if segments.get(sep + 1)? != "merge_requests" {
        return None;
    }

    let iid = segments.get(sep + 2)?.parse().ok()?;
    if sep == 0 {
        return None;
    }

    let project_path = segments[..sep].join("/");

    Some(ReviewTarget::GitLab {
        host: host.to_string(),
        project_path,
        iid,
        url: input.to_string(),
    })
}
