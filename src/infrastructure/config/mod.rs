//! 사용자 설정(JSON) 로딩/병합 모듈.
//! 여러 경로의 설정을 우선순위대로 병합하고, 실행 진단용 정보를 함께 제공한다.

mod inspection;
mod loader;
mod resolve;
mod utils;

use std::path::PathBuf;

use anyhow::Result;

pub use crate::application::config::{
    Config, DefaultsConfig, HostConfig, ProviderCommandSpec, ProviderConfig, ProvidersConfig,
    DEFAULT_SYSTEM_PROMPT,
};
pub use inspection::{
    ConfigInspection, EffectiveDefaults, HostInspection, ProviderInspection, ProvidersInspection,
};
pub use loader::config_paths;
pub use resolve::{
    ProviderCredentialResolution, resolve_host_token, resolve_provider_api_key,
    provider_api_key_source_label,
};
pub use utils::command_exists;

/// 병합된 최종 설정을 로딩한다.
pub fn load() -> Result<Config> {
    Ok(loader::load_merged_config()?.config)
}

/// 디버깅/진단용 설정 정보를 구성한다.
pub fn inspect() -> Result<ConfigInspection> {
    let loaded = loader::load_merged_config()?;
    Ok(ConfigInspection::from_loaded(loaded))
}

/// 설정 진단 결과를 사람이 읽기 쉬운 JSON으로 반환한다.
pub fn inspect_pretty_json() -> Result<String> {
    Ok(serde_json::to_string_pretty(&inspect()?)?)
}

/// 편집 대상 설정 파일 경로를 반환한다.
pub fn editable_path() -> Result<PathBuf> {
    loader::editable_config_path()
}
