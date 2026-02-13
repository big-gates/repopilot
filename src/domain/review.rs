//! 리뷰 도메인 엔티티/값 객체.

use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct RunOptions {
    pub url: String,
    pub dry_run: bool,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct ReviewComment {
    pub id: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct ReviewRequest {
    pub target_url: String,
    pub head_sha: String,
    pub diff: String,
    pub system_prompt: String,
    pub comment_language: CommentLanguage,
}

/// 리뷰 결과 출력 언어 정책.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentLanguage {
    Korean,
    English,
}

impl CommentLanguage {
    /// 설정 문자열을 언어 정책으로 변환한다.
    /// 지원값: ko/korean, en/english (미지정/알수없음은 ko 기본값)
    pub fn from_config(value: Option<&str>) -> Self {
        let Some(raw) = value else {
            return Self::Korean;
        };

        match raw.trim().to_ascii_lowercase().as_str() {
            "en" | "english" => Self::English,
            "ko" | "kr" | "korean" => Self::Korean,
            _ => Self::Korean,
        }
    }

    /// 프롬프트에 넣을 출력 언어 지시문(영문).
    pub fn prompt_instruction(self) -> &'static str {
        match self {
            Self::Korean => {
                "Write the final answer in Korean only. Do not use English headings or body text."
            }
            Self::English => {
                "Write the final answer in English only. Do not use Korean headings or body text."
            }
        }
    }

    /// inspection 출력용 코드값.
    pub fn code(self) -> &'static str {
        match self {
            Self::Korean => "ko",
            Self::English => "en",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

impl TokenUsage {
    pub fn add_from(&mut self, other: &TokenUsage) {
        self.prompt_tokens = sum_optional(self.prompt_tokens, other.prompt_tokens);
        self.completion_tokens = sum_optional(self.completion_tokens, other.completion_tokens);
        self.total_tokens = sum_optional(self.total_tokens, other.total_tokens);
    }
}

#[derive(Debug, Clone)]
pub struct ProviderResponse {
    pub content: String,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone)]
pub struct ProviderRun {
    pub id: String,
    pub name: String,
    pub body: String,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone)]
pub struct AgentComment {
    pub provider_id: String,
    pub provider_name: String,
    pub body: String,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone)]
pub struct AgentReaction {
    pub provider_name: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct ReviewMarkers {
    pub final_marker: String,
    pub claim_marker: String,
}

pub type UsageTotals = BTreeMap<String, (String, TokenUsage)>;

fn sum_optional(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x + y),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}
