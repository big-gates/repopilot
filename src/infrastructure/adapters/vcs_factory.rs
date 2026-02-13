//! VCS 게이트웨이 포트 구현 어댑터.

use anyhow::Result;
use async_trait::async_trait;

use crate::application::ports::{VcsFactory, VcsGateway};
use crate::domain::review::ReviewComment;
use crate::domain::target::ReviewTarget;
use crate::infrastructure::{config, vcs};

/// VCS 게이트웨이 팩토리 어댑터.
pub struct VcsFactoryAdapter;

impl VcsFactory for VcsFactoryAdapter {
    fn build(
        &self,
        target: &ReviewTarget,
        host_cfg: Option<&config::HostConfig>,
        token: Option<String>,
    ) -> Box<dyn VcsGateway> {
        Box::new(VcsGatewayAdapter {
            inner: vcs::build_vcs_client(target, host_cfg, token),
        })
    }
}

/// 인프라 VCS Provider를 애플리케이션 포트로 감싸는 래퍼.
struct VcsGatewayAdapter {
    inner: Box<dyn vcs::VcsProvider>,
}

#[async_trait]
impl VcsGateway for VcsGatewayAdapter {
    async fn fetch_head_sha(&self) -> Result<String> {
        self.inner.fetch_head_sha().await
    }

    async fn fetch_diff(&self, max_bytes: usize) -> Result<String> {
        self.inner.fetch_diff(max_bytes).await
    }

    async fn list_comments(&self) -> Result<Vec<ReviewComment>> {
        self.inner.list_comments().await
    }

    async fn create_comment(&self, body: &str) -> Result<ReviewComment> {
        self.inner.create_comment(body).await
    }

    async fn update_comment(&self, comment_id: &str, body: &str) -> Result<ReviewComment> {
        self.inner.update_comment(comment_id, body).await
    }
}
