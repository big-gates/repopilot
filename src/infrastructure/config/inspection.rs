//! 적용 설정 진단(inspection) 뷰 모델.

use std::collections::BTreeMap;

use serde::Serialize;

use super::loader::LoadedConfig;
use super::resolve::{resolve_host_token, resolve_provider_api_key};
use super::utils::command_exists;
use crate::application::config::{DefaultsConfig, HostConfig, ProviderConfig};

#[derive(Debug, Clone, Serialize)]
pub struct ConfigInspection {
    pub searched_paths: Vec<String>,
    pub loaded_paths: Vec<String>,
    pub defaults: DefaultsConfig,
    pub effective_defaults: EffectiveDefaults,
    pub hosts: BTreeMap<String, HostInspection>,
    pub providers: ProvidersInspection,
}

#[derive(Debug, Clone, Serialize)]
pub struct EffectiveDefaults {
    pub max_diff_bytes: usize,
    pub system_prompt: String,
    pub review_guide_path: Option<String>,
    pub comment_language: String,
    pub update_check_url: Option<String>,
    pub update_download_url: Option<String>,
    pub update_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostInspection {
    pub token_source: Option<String>,
    pub token_resolved: bool,
    pub api_base: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProvidersInspection {
    pub openai: Option<ProviderInspection>,
    pub anthropic: Option<ProviderInspection>,
    pub gemini: Option<ProviderInspection>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderInspection {
    pub enabled: bool,
    pub resolved_mode: String,
    pub runnable: bool,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub use_stdin: bool,
    pub command_available: bool,
    pub api_key_source: Option<String>,
    pub api_key_resolved: bool,
}

impl ConfigInspection {
    pub(crate) fn from_loaded(loaded: LoadedConfig) -> Self {
        let mut hosts = BTreeMap::new();
        for (host, cfg) in &loaded.config.hosts {
            hosts.insert(host.clone(), host_inspection(cfg));
        }

        Self {
            searched_paths: loaded
                .searched_paths
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
            loaded_paths: loaded
                .loaded_paths
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
            defaults: loaded.config.defaults.clone(),
            effective_defaults: EffectiveDefaults {
                max_diff_bytes: loaded.config.max_diff_bytes(),
                system_prompt: loaded.config.system_prompt(),
                review_guide_path: loaded.config.defaults.review_guide_path.clone(),
                comment_language: loaded.config.comment_language().code().to_string(),
                update_check_url: loaded.config.defaults.update_check_url.clone(),
                update_download_url: loaded.config.defaults.update_download_url.clone(),
                update_timeout_ms: loaded.config.defaults.update_timeout_ms.unwrap_or(1200),
            },
            hosts,
            providers: ProvidersInspection {
                openai: loaded
                    .config
                    .providers
                    .openai
                    .as_ref()
                    .map(|cfg| ProviderInspection::from_config(cfg, "codex")),
                anthropic: loaded
                    .config
                    .providers
                    .anthropic
                    .as_ref()
                    .map(|cfg| ProviderInspection::from_config(cfg, "claude")),
                gemini: loaded
                    .config
                    .providers
                    .gemini
                    .as_ref()
                    .map(|cfg| ProviderInspection::from_config(cfg, "gemini")),
            },
        }
    }
}

impl ProviderInspection {
    fn from_config(cfg: &ProviderConfig, default_command: &str) -> Self {
        let enabled = cfg.is_enabled();
        let api_resolution = resolve_provider_api_key(cfg);
        let api_ready = api_resolution.credential.is_some();
        let command_spec = cfg.command_spec(default_command);
        let command = command_spec.as_ref().map(|s| s.command.clone());
        let args = command_spec
            .as_ref()
            .map(|s| s.args.clone())
            .unwrap_or_default();
        let use_stdin = command_spec.as_ref().map(|s| s.use_stdin).unwrap_or(true);

        let command_available = command
            .as_ref()
            .map(|c| command_exists(c))
            .unwrap_or(false);
        let resolved_mode = if !enabled {
            "disabled"
        } else if api_ready {
            "api"
        } else {
            "cli"
        };
        let runnable = enabled && (api_ready || command_available);

        Self {
            enabled,
            resolved_mode: resolved_mode.to_string(),
            runnable,
            command,
            args,
            use_stdin,
            command_available,
            api_key_source: api_resolution.source,
            api_key_resolved: api_ready,
        }
    }
}

fn host_inspection(cfg: &HostConfig) -> HostInspection {
    let token_resolution = resolve_host_token(Some(cfg)).ok();
    HostInspection {
        token_source: token_resolution.as_ref().and_then(|r| r.source.clone()),
        token_resolved: token_resolution
            .as_ref()
            .and_then(|r| r.token.as_ref())
            .is_some(),
        api_base: cfg.api_base.clone(),
    }
}
