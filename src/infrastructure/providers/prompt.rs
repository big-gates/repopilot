//! Provider 공통 프롬프트 구성.

use crate::domain::review::ReviewRequest;

/// 1차 리뷰용 사용자 프롬프트를 생성한다.
pub fn build_user_prompt(request: &ReviewRequest) -> String {
    format!(
        "Target URL: {}\nHead SHA: {}\n\nPlease review this diff and return concise Markdown findings.\n\n```diff\n{}\n```",
        request.target_url, request.head_sha, request.diff
    )
}
