//! GitHub API 연동 구현.

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::{Client, Method, RequestBuilder};
use serde::Deserialize;
use serde_json::json;

use super::{ReviewComment, VcsProvider};

pub struct GitHubClient {
    client: Client,
    host: String,
    owner: String,
    repo: String,
    number: u64,
    token: Option<String>,
    api_base: Option<String>,
}

impl GitHubClient {
    /// GitHub 대상 클라이언트를 생성한다.
    pub fn new(
        host: String,
        owner: String,
        repo: String,
        number: u64,
        token: Option<String>,
        api_base: Option<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            host,
            owner,
            repo,
            number,
            token,
            api_base,
        }
    }

    fn api_base(&self) -> String {
        // github.com은 공개 API, 그 외는 Enterprise 기본 경로를 사용한다.
        if let Some(base) = &self.api_base {
            return base.trim_end_matches('/').to_string();
        }
        if self.host == "github.com" {
            "https://api.github.com".to_string()
        } else {
            format!("https://{}/api/v3", self.host)
        }
    }

    fn pulls_endpoint(&self) -> String {
        format!(
            "{}/repos/{}/{}/pulls/{}",
            self.api_base(),
            self.owner,
            self.repo,
            self.number
        )
    }

    fn issue_comments_endpoint(&self) -> String {
        format!(
            "{}/repos/{}/{}/issues/{}/comments",
            self.api_base(),
            self.owner,
            self.repo,
            self.number
        )
    }

    fn issue_comment_endpoint(&self, comment_id: &str) -> String {
        format!(
            "{}/repos/{}/{}/issues/comments/{}",
            self.api_base(),
            self.owner,
            self.repo,
            comment_id
        )
    }

    fn request(&self, method: Method, url: String) -> RequestBuilder {
        // 공통 헤더/인증 적용.
        let req = self
            .client
            .request(method, url)
            .header("User-Agent", "prpilot")
            .header("Accept", "application/vnd.github+json");

        if let Some(token) = &self.token {
            req.bearer_auth(token)
        } else {
            req
        }
    }
}

#[derive(Debug, Deserialize)]
struct PullResponse {
    head: PullHead,
}

#[derive(Debug, Deserialize)]
struct PullHead {
    sha: String,
}

#[derive(Debug, Deserialize)]
struct IssueCommentResponse {
    id: u64,
    body: String,
}

#[async_trait]
impl VcsProvider for GitHubClient {
    async fn fetch_head_sha(&self) -> Result<String> {
        let resp = self
            .request(Method::GET, self.pulls_endpoint())
            .send()
            .await
            .context("github: failed to fetch PR")?;

        let status = resp.status();
        let body = resp.text().await.context("github: failed to read PR body")?;
        if !status.is_success() {
            anyhow::bail!("github: failed to fetch PR metadata ({status}): {body}");
        }

        let pr: PullResponse = serde_json::from_str(&body).context("github: invalid PR JSON")?;
        Ok(pr.head.sha)
    }

    async fn fetch_diff(&self) -> Result<String> {
        // PR endpoint에 diff Accept 헤더를 적용해 unified diff를 가져온다.
        let mut req = self
            .client
            .get(self.pulls_endpoint())
            .header("User-Agent", "prpilot")
            .header("Accept", "application/vnd.github.v3.diff");
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }

        let resp = req
            .send()
            .await
            .context("github: failed to fetch PR diff")?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .context("github: failed to read PR diff body")?;

        if !status.is_success() {
            anyhow::bail!("github: failed to fetch PR diff ({status}): {body}");
        }

        Ok(body)
    }

    async fn list_comments(&self) -> Result<Vec<ReviewComment>> {
        let resp = self
            .request(Method::GET, self.issue_comments_endpoint())
            .send()
            .await
            .context("github: failed to list comments")?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .context("github: failed to read comments body")?;

        if !status.is_success() {
            anyhow::bail!("github: failed to list comments ({status}): {body}");
        }

        let comments: Vec<IssueCommentResponse> =
            serde_json::from_str(&body).context("github: invalid comments JSON")?;

        Ok(comments
            .into_iter()
            .map(|c| ReviewComment {
                id: c.id.to_string(),
                body: c.body,
            })
            .collect())
    }

    async fn create_comment(&self, body: &str) -> Result<ReviewComment> {
        let resp = self
            .request(Method::POST, self.issue_comments_endpoint())
            .json(&json!({ "body": body }))
            .send()
            .await
            .context("github: failed to create comment")?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .context("github: failed to read create-comment body")?;

        if !status.is_success() {
            anyhow::bail!("github: failed to create comment ({status}): {body}");
        }

        let comment: IssueCommentResponse =
            serde_json::from_str(&body).context("github: invalid create-comment JSON")?;
        Ok(ReviewComment {
            id: comment.id.to_string(),
            body: comment.body,
        })
    }

    async fn update_comment(&self, comment_id: &str, body: &str) -> Result<ReviewComment> {
        let resp = self
            .request(Method::PATCH, self.issue_comment_endpoint(comment_id))
            .json(&json!({ "body": body }))
            .send()
            .await
            .context("github: failed to update comment")?;

        let status = resp.status();
        let response_body = resp
            .text()
            .await
            .context("github: failed to read update-comment body")?;

        if !status.is_success() {
            anyhow::bail!("github: failed to update comment ({status}): {response_body}");
        }

        let comment: IssueCommentResponse =
            serde_json::from_str(&response_body).context("github: invalid update-comment JSON")?;

        Ok(ReviewComment {
            id: comment.id.to_string(),
            body: comment.body,
        })
    }
}
