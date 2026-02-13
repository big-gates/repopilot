//! 설정 저장소 포트 구현 어댑터.

use std::path::PathBuf;

use anyhow::Result;

use crate::application::ports::ConfigRepository;
use crate::infrastructure::config;

/// JSON 기반 설정 저장소 어댑터.
pub struct JsonConfigRepository;

impl ConfigRepository for JsonConfigRepository {
    fn load(&self) -> Result<config::Config> {
        config::Config::load()
    }

    fn inspect_pretty_json(&self) -> Result<String> {
        config::Config::inspect_pretty_json()
    }

    fn editable_config_path(&self) -> Result<PathBuf> {
        config::Config::editable_path()
    }
}
