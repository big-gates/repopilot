//! 설정 파일 탐색/병합 로더.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::types::Config;

#[derive(Debug, Clone)]
pub(crate) struct LoadedConfig {
    pub config: Config,
    pub searched_paths: Vec<PathBuf>,
    pub loaded_paths: Vec<PathBuf>,
}

/// 우선순위 경로를 순회해 JSON 설정을 병합한다.
pub(crate) fn load_merged_config() -> Result<LoadedConfig> {
    // 낮은 우선순위에서 높은 우선순위 순서로 병합한다.
    let mut merged = Config::default();
    let mut loaded_paths = Vec::new();
    let paths = config_paths();

    if let Ok(path) = env::var("PRPILOT_CONFIG") {
        let explicit = Path::new(&path);
        if !explicit.exists() {
            bail!(
                "PRPILOT_CONFIG is set but file does not exist: {}",
                explicit.display()
            );
        }
    }

    for path in &paths {
        if !path.exists() {
            continue;
        }

        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read config at {}", path.display()))?;
        let parsed: Config = serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse JSON in {}", path.display()))?;
        merged.merge_from(parsed);
        loaded_paths.push(path.to_path_buf());
    }

    if loaded_paths.is_empty() {
        bail!(
            "no config file found. looked in: {}",
            paths
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    Ok(LoadedConfig {
        config: merged,
        searched_paths: paths,
        loaded_paths,
    })
}

/// 기본 + 사용자 + 프로젝트 + 명시 경로 순으로 병합 경로를 구성한다.
pub fn config_paths() -> Vec<PathBuf> {
    // 낮은 우선순위 -> 높은 우선순위 순서로 병합됨.
    let mut paths = vec![PathBuf::from("/etc/prpilot/config.json")];

    if let Some(base) = dirs::config_dir() {
        paths.push(base.join("prpilot").join("config.json"));
    }

    paths.push(PathBuf::from(".prpilot/config.json"));
    paths.push(PathBuf::from("prpilot.config.json"));

    if let Ok(path) = env::var("PRPILOT_CONFIG") {
        paths.push(Path::new(&path).to_path_buf());
    }

    dedup_paths(paths)
}

fn dedup_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for p in paths {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}
