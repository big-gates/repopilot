//! 설정 스키마와 병합/해석 규칙.

use std::collections::HashMap;
use std::env;
use std::fs;

use anyhow::{Context, Result};
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
    /// 리뷰 기본 시스템 프롬프트
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
    pub token: Option<String>,
    pub token_env: Option<String>,
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

    /// 기본 시스템 프롬프트 + review guide 파일 내용을 합쳐 반환한다.
    pub fn resolved_system_prompt(&self) -> Result<String> {
        let mut prompt = self.system_prompt();

        if let Some(path) = &self.defaults.review_guide_path {
            let guide_raw = fs::read_to_string(path)
                .with_context(|| format!("failed to read review guide file at {}", path))?;
            let guide = guide_raw.trim();
            if !guide.is_empty() {
                prompt.push_str("\n\nReview guide (must follow):\n");
                prompt.push_str(guide);
            }
        }

        Ok(prompt)
    }

    pub fn host_config(&self, host: &str) -> Option<&HostConfig> {
        self.hosts.get(host)
    }

    /// 후순위(나중 파일) 값으로 덮어쓰는 병합 규칙.
    pub(crate) fn merge_from(&mut self, other: Config) {
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
    pub(crate) fn merge_from(&mut self, other: DefaultsConfig) {
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
    /// host 토큰은 `token` 우선, 없으면 `token_env`를 조회한다.
    pub fn resolve_token(&self) -> Option<String> {
        if let Some(token) = &self.token {
            return Some(token.clone());
        }
        let env_name = self.token_env.as_ref()?;
        env::var(env_name).ok().filter(|v| !v.trim().is_empty())
    }

    pub(crate) fn merge_from(&mut self, other: HostConfig) {
        if other.token.is_some() {
            self.token = other.token;
        }
        if other.token_env.is_some() {
            self.token_env = other.token_env;
        }
        if other.api_base.is_some() {
            self.api_base = other.api_base;
        }
    }

    pub(crate) fn token_source_label(&self) -> Option<String> {
        if self.token.is_some() {
            return Some("inline".to_string());
        }
        if let Some(env_name) = &self.token_env {
            return if env::var(env_name)
                .ok()
                .filter(|v| !v.trim().is_empty())
                .is_some()
            {
                Some(format!("env:{env_name}"))
            } else {
                Some(format!("env:{env_name} (missing)"))
            };
        }
        None
    }
}

impl ProviderConfig {
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
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

    pub fn resolve_api_key(&self) -> Option<String> {
        if let Some(key) = &self.api_key {
            return Some(key.clone());
        }
        let env_name = self.api_key_env.as_ref()?;
        env::var(env_name).ok().filter(|v| !v.trim().is_empty())
    }

    pub(crate) fn merge_from(&mut self, other: ProviderConfig) {
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

    pub(crate) fn api_key_source_label(&self) -> Option<String> {
        if self.api_key.is_some() {
            return Some("inline".to_string());
        }
        if let Some(env_name) = &self.api_key_env {
            return if env::var(env_name)
                .ok()
                .filter(|v| !v.trim().is_empty())
                .is_some()
            {
                Some(format!("env:{env_name}"))
            } else {
                Some(format!("env:{env_name} (missing)"))
            };
        }
        None
    }
}

impl ProvidersConfig {
    pub(crate) fn merge_from(&mut self, other: ProvidersConfig) {
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
