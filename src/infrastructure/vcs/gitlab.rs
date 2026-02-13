//! GitLab API 연동 구현.

use anyhow::{Context, Result};
use async_trait::async_trait;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::{Client, Method, RequestBuilder};
use serde::Deserialize;
use serde_json::json;

use super::{ReviewComment, VcsProvider, truncate_diff};

pub struct GitLabClient {
    client: Client,
    host: String,
    project_path: String,
    iid: u64,
    token: Option<String>,
    api_base: Option<String>,
}

impl GitLabClient {
    /// GitLab 대상 클라이언트를 생성한다.
    pub fn new(
        host: String,
        project_path: String,
        iid: u64,
        token: Option<String>,
        api_base: Option<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            host,
            project_path,
            iid,
            token,
            api_base,
        }
    }

    fn api_base(&self) -> String {
        // gitlab.com은 공개 API, 그 외는 self-hosted 기본 경로를 사용한다.
        if let Some(base) = &self.api_base {
            return base.trim_end_matches('/').to_string();
        }
        if self.host == "gitlab.com" {
            "https://gitlab.com/api/v4".to_string()
        } else {
            format!("https://{}/api/v4", self.host)
        }
    }

    fn encoded_project_path(&self) -> String {
        // /projects/{path} API 규격에 맞춰 경로를 URL 인코딩한다.
        utf8_percent_encode(&self.project_path, NON_ALPHANUMERIC).to_string()
    }

    fn merge_request_endpoint(&self) -> String {
        format!(
            "{}/projects/{}/merge_requests/{}",
            self.api_base(),
            self.encoded_project_path(),
            self.iid
        )
    }

    fn merge_request_changes_endpoint(&self) -> String {
        format!("{}/changes", self.merge_request_endpoint())
    }

    fn notes_endpoint(&self) -> String {
        format!("{}/notes", self.merge_request_endpoint())
    }

    fn note_endpoint(&self, note_id: &str) -> String {
        format!("{}/{}", self.notes_endpoint(), note_id)
    }

    fn request(&self, method: Method, url: String) -> RequestBuilder {
        // GitLab 토큰 헤더(`PRIVATE-TOKEN`)를 공통 적용한다.
        let req = self.client.request(method, url);
        if let Some(token) = &self.token {
            req.header("PRIVATE-TOKEN", token)
        } else {
            req
        }
    }
}

#[derive(Debug, Deserialize)]
struct MergeRequestResponse {
    sha: Option<String>,
    diff_refs: Option<DiffRefs>,
}

#[derive(Debug, Deserialize)]
struct DiffRefs {
    head_sha: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MergeRequestChangesResponse {
    changes: Vec<MergeRequestChange>,
}

#[derive(Debug, Deserialize)]
struct MergeRequestChange {
    diff: String,
}

#[derive(Debug, Deserialize)]
struct NoteResponse {
    id: u64,
    body: String,
}

#[async_trait]
impl VcsProvider for GitLabClient {
    async fn fetch_head_sha(&self) -> Result<String> {
        let resp = self
            .request(Method::GET, self.merge_request_endpoint())
            .send()
            .await
            .context("gitlab: failed to fetch MR")?;

        let status = resp.status();
        let body = resp.text().await.context("gitlab: failed to read MR body")?;

        if !status.is_success() {
            anyhow::bail!("gitlab: failed to fetch MR metadata ({status}): {body}");
        }

        let mr: MergeRequestResponse =
            serde_json::from_str(&body).context("gitlab: invalid MR JSON")?;

        if let Some(sha) = mr.sha {
            return Ok(sha);
        }
        if let Some(refs) = mr.diff_refs && let Some(sha) = refs.head_sha {
            return Ok(sha);
        }

        anyhow::bail!("gitlab: MR response missing sha and diff_refs.head_sha")
    }

    async fn fetch_diff(&self, max_bytes: usize) -> Result<String> {
        // changes API의 개별 diff를 이어붙여 unified diff처럼 사용한다.
        let resp = self
            .request(Method::GET, self.merge_request_changes_endpoint())
            .send()
            .await
            .context("gitlab: failed to fetch MR changes")?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .context("gitlab: failed to read MR changes body")?;

        if !status.is_success() {
            anyhow::bail!("gitlab: failed to fetch MR changes ({status}): {body}");
        }

        let changes: MergeRequestChangesResponse =
            serde_json::from_str(&body).context("gitlab: invalid MR changes JSON")?;

        let joined = changes
            .changes
            .into_iter()
            .map(|c| c.diff)
            .collect::<Vec<_>>()
            .join("\n");

        Ok(truncate_diff(joined, max_bytes))
    }

    async fn list_comments(&self) -> Result<Vec<ReviewComment>> {
        let resp = self
            .request(Method::GET, self.notes_endpoint())
            .send()
            .await
            .context("gitlab: failed to list notes")?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .context("gitlab: failed to read notes body")?;

        if !status.is_success() {
            anyhow::bail!("gitlab: failed to list notes ({status}): {body}");
        }

        let notes: Vec<NoteResponse> = serde_json::from_str(&body).context("gitlab: invalid notes JSON")?;

        Ok(notes
            .into_iter()
            .map(|n| ReviewComment {
                id: n.id.to_string(),
                body: n.body,
            })
            .collect())
    }

    async fn create_comment(&self, body: &str) -> Result<ReviewComment> {
        let resp = self
            .request(Method::POST, self.notes_endpoint())
            .json(&json!({ "body": body }))
            .send()
            .await
            .context("gitlab: failed to create note")?;

        let status = resp.status();
        let response_body = resp
            .text()
            .await
            .context("gitlab: failed to read create-note body")?;

        if !status.is_success() {
            anyhow::bail!("gitlab: failed to create note ({status}): {response_body}");
        }

        let note: NoteResponse =
            serde_json::from_str(&response_body).context("gitlab: invalid create-note JSON")?;

        Ok(ReviewComment {
            id: note.id.to_string(),
            body: note.body,
        })
    }

    async fn update_comment(&self, comment_id: &str, body: &str) -> Result<ReviewComment> {
        let resp = self
            .request(Method::PUT, self.note_endpoint(comment_id))
            .json(&json!({ "body": body }))
            .send()
            .await
            .context("gitlab: failed to update note")?;

        let status = resp.status();
        let response_body = resp
            .text()
            .await
            .context("gitlab: failed to read update-note body")?;

        if !status.is_success() {
            anyhow::bail!("gitlab: failed to update note ({status}): {response_body}");
        }

        let note: NoteResponse =
            serde_json::from_str(&response_body).context("gitlab: invalid update-note JSON")?;

        Ok(ReviewComment {
            id: note.id.to_string(),
            body: note.body,
        })
    }
}
