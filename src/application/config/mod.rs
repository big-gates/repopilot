//! 애플리케이션이 사용하는 설정 스키마(순수 데이터).
//!
//! 주의: 파일/환경변수/프로세스 접근은 `infrastructure`에서만 수행한다.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::domain::review::CommentLanguage;

pub const DEFAULT_MAX_DIFF_BYTES: usize = 120_000;
pub const DEFAULT_SYSTEM_PROMPT: &str =
    "You are a strict senior code reviewer. Output Markdown with sections: Critical, Major, Minor, Suggestions.";

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    /// 전역 기본값
    #[serde(default)]
    pub defaults: DefaultsConfig,
    /// VCS 호스트별 인증/엔드포인트 설정
    #[serde(default)]
    pub hosts: HashMap<String, HostConfig>,
    /// provider 실행 설정
    #[serde(default)]
    pub providers: ProvidersConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct DefaultsConfig {
    /// diff 최대 바이트
    pub max_diff_bytes: Option<usize>,
    /// 리뷰 기본 시스템 프롬프트(이미 resolve된 값일 수 있음)
    pub system_prompt: Option<String>,
    /// 리뷰 지침 markdown 파일 경로
    pub review_guide_path: Option<String>,
    /// 리뷰 코멘트 출력 언어(ko/en)
    pub comment_language: Option<String>,
    /// 최신 버전 확인용 엔드포인트 URL (plain text 또는 JSON)
    pub update_check_url: Option<String>,
    /// 업데이트 안내 시 표시할 다운로드 URL 힌트
    pub update_download_url: Option<String>,
    /// 업데이트 확인 타임아웃(ms)
    pub update_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HostConfig {
    /// 고정 토큰(민감정보: 권장하지 않음)
    pub token: Option<String>,
    /// 토큰을 읽을 환경변수 이름
    pub token_env: Option<String>,
    /// 토큰을 stdout으로 출력하는 커맨드(예: ["gh","auth","token"])
    pub token_command: Option<Vec<String>>,
    /// API base URL override(선택)
    pub api_base: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ProvidersConfig {
    pub openai: Option<ProviderConfig>,
    pub anthropic: Option<ProviderConfig>,
    pub gemini: Option<ProviderConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ProviderConfig {
    /// provider 활성화 여부(기본 true)
    pub enabled: Option<bool>,
    /// 실행할 로컬 명령
    pub command: Option<String>,
    /// 명령 인자
    pub args: Option<Vec<String>>,
    /// 프롬프트를 stdin으로 전달할지 여부(기본 true)
    pub use_stdin: Option<bool>,
    /// CLI 모드에서 인증이 필요할 때 자동으로 로그인 시도할지 여부(기본 true)
    pub auto_auth: Option<bool>,
    /// OAuth/로그인용 커맨드 (예: ["codex","login"], ["claude","auth","login"], ["gemini"])
    pub auth_command: Option<Vec<String>>,

    /// API 모드에서 사용할 모델 식별자(선택)
    pub model: Option<String>,
    /// API 모드 베이스 URL(선택)
    pub api_base: Option<String>,
    /// API 모드 인증 키/토큰(직접값)
    pub api_key: Option<String>,
    /// API 모드 인증 키/토큰을 읽을 환경변수 이름
    pub api_key_env: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProviderCommandSpec {
    pub command: String,
    pub args: Vec<String>,
    pub use_stdin: bool,
}

impl Config {
    pub fn max_diff_bytes(&self) -> usize {
        self.defaults
            .max_diff_bytes
            .unwrap_or(DEFAULT_MAX_DIFF_BYTES)
    }

    pub fn system_prompt(&self) -> String {
        self.defaults
            .system_prompt
            .clone()
            .unwrap_or_else(|| DEFAULT_SYSTEM_PROMPT.to_string())
    }

    /// 리뷰 코멘트 출력 언어를 해석한다.
    pub fn comment_language(&self) -> CommentLanguage {
        CommentLanguage::from_config(self.defaults.comment_language.as_deref())
    }

    pub fn host_config(&self, host: &str) -> Option<&HostConfig> {
        self.hosts.get(host)
    }

    /// 후순위(나중 파일) 값으로 덮어쓰는 병합 규칙.
    pub fn merge_from(&mut self, other: Config) {
        self.defaults.merge_from(other.defaults);

        for (host, incoming) in other.hosts {
            if let Some(existing) = self.hosts.get_mut(&host) {
                existing.merge_from(incoming);
            } else {
                self.hosts.insert(host, incoming);
            }
        }

        self.providers.merge_from(other.providers);
    }
}

impl DefaultsConfig {
    pub fn merge_from(&mut self, other: DefaultsConfig) {
        if other.max_diff_bytes.is_some() {
            self.max_diff_bytes = other.max_diff_bytes;
        }
        if other.system_prompt.is_some() {
            self.system_prompt = other.system_prompt;
        }
        if other.review_guide_path.is_some() {
            self.review_guide_path = other.review_guide_path;
        }
        if other.comment_language.is_some() {
            self.comment_language = other.comment_language;
        }
        if other.update_check_url.is_some() {
            self.update_check_url = other.update_check_url;
        }
        if other.update_download_url.is_some() {
            self.update_download_url = other.update_download_url;
        }
        if other.update_timeout_ms.is_some() {
            self.update_timeout_ms = other.update_timeout_ms;
        }
    }
}

impl HostConfig {
    pub fn merge_from(&mut self, other: HostConfig) {
        if other.token.is_some() {
            self.token = other.token;
        }
        if other.token_env.is_some() {
            self.token_env = other.token_env;
        }
        if other.token_command.is_some() {
            self.token_command = other.token_command;
        }
        if other.api_base.is_some() {
            self.api_base = other.api_base;
        }
    }
}

impl ProviderConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }

    pub fn auto_auth(&self) -> bool {
        self.auto_auth.unwrap_or(true)
    }

    /// provider 실행 사양(명령/인자/stdin)을 정규화한다.
    pub fn command_spec(&self, default_command: &str) -> Option<ProviderCommandSpec> {
        if !self.is_enabled() {
            return None;
        }

        Some(ProviderCommandSpec {
            command: self
                .command
                .clone()
                .unwrap_or_else(|| default_command.to_string()),
            args: self.args.clone().unwrap_or_default(),
            use_stdin: self.use_stdin.unwrap_or(true),
        })
    }

    pub fn merge_from(&mut self, other: ProviderConfig) {
        if other.enabled.is_some() {
            self.enabled = other.enabled;
        }
        if other.command.is_some() {
            self.command = other.command;
        }
        if other.args.is_some() {
            self.args = other.args;
        }
        if other.use_stdin.is_some() {
            self.use_stdin = other.use_stdin;
        }
        if other.auto_auth.is_some() {
            self.auto_auth = other.auto_auth;
        }
        if other.auth_command.is_some() {
            self.auth_command = other.auth_command;
        }

        if other.api_key.is_some() {
            self.api_key = other.api_key;
        }
        if other.api_key_env.is_some() {
            self.api_key_env = other.api_key_env;
        }
        if other.model.is_some() {
            self.model = other.model;
        }
        if other.api_base.is_some() {
            self.api_base = other.api_base;
        }
    }
}

impl ProvidersConfig {
    pub fn merge_from(&mut self, other: ProvidersConfig) {
        merge_provider_config(&mut self.openai, other.openai);
        merge_provider_config(&mut self.anthropic, other.anthropic);
        merge_provider_config(&mut self.gemini, other.gemini);
    }
}

fn merge_provider_config(target: &mut Option<ProviderConfig>, incoming: Option<ProviderConfig>) {
    match (target.as_mut(), incoming) {
        (Some(existing), Some(next)) => existing.merge_from(next),
        (None, Some(next)) => *target = Some(next),
        _ => {}
    }
}
