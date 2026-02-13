//! Provider 공통 프롬프트 구성.

use crate::domain::review::ReviewRequest;

/// 1차 리뷰용 시스템+사용자 통합 프롬프트를 생성한다.
pub fn build_primary_prompt(request: &ReviewRequest) -> String {
    format!(
        "System instructions:\n{}\n\nOutput language requirement:\n{}\n\n{}",
        request.system_prompt,
        request.comment_language.prompt_instruction(),
        build_user_prompt(request)
    )
}

/// 1차 리뷰용 사용자 프롬프트를 생성한다.
pub fn build_user_prompt(request: &ReviewRequest) -> String {
    format!(
        "Target URL: {}\nHead SHA: {}\n\nReview the diff and report key issues in concise Markdown.\nUse sections in this order: Critical, Major, Minor, Suggestions.\n\n```diff\n{}\n```",
        request.target_url, request.head_sha, request.diff
    )
}
