//! VCS 추상화 계층.
//! GitHub/GitLab별 구현을 공통 인터페이스로 묶는다.

pub mod github;
pub mod gitlab;

use anyhow::Result;
use async_trait::async_trait;

use crate::domain::review::ReviewComment;
use crate::domain::target::ReviewTarget;
use crate::infrastructure::config::HostConfig;

#[async_trait]
pub trait VcsProvider: Send + Sync {
    /// PR/MR의 현재 HEAD SHA 조회
    async fn fetch_head_sha(&self) -> Result<String>;
    /// API 기반 diff 조회
    async fn fetch_diff(&self, max_bytes: usize) -> Result<String>;
    /// 기존 코멘트/노트 조회
    async fn list_comments(&self) -> Result<Vec<ReviewComment>>;
    /// 코멘트/노트 생성
    async fn create_comment(&self, body: &str) -> Result<ReviewComment>;
    /// 코멘트/노트 수정
    async fn update_comment(&self, comment_id: &str, body: &str) -> Result<ReviewComment>;
}

pub fn build_vcs_client(
    target: &ReviewTarget,
    host_cfg: Option<&HostConfig>,
    token: Option<String>,
) -> Box<dyn VcsProvider> {
    // URL 해석 결과에 따라 적절한 VCS 구현체를 선택한다.
    let api_base = host_cfg.and_then(|h| h.api_base.clone());

    match target {
        ReviewTarget::GitHub {
            host,
            owner,
            repo,
            number,
            ..
        } => Box::new(github::GitHubClient::new(
            host.clone(),
            owner.clone(),
            repo.clone(),
            *number,
            token,
            api_base,
        )),
        ReviewTarget::GitLab {
            host,
            project_path,
            iid,
            ..
        } => Box::new(gitlab::GitLabClient::new(
            host.clone(),
            project_path.clone(),
            *iid,
            token,
            api_base,
        )),
    }
}

pub fn truncate_diff(mut diff: String, max_bytes: usize) -> String {
    // UTF-8 경계를 지키면서 diff를 안전하게 자른다.
    if diff.len() <= max_bytes {
        return diff;
    }

    let mut cutoff = max_bytes;
    while cutoff > 0 && !diff.is_char_boundary(cutoff) {
        cutoff -= 1;
    }

    diff.truncate(cutoff);
    diff.push_str("\n... (diff truncated)\n");
    diff
}
