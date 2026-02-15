//! 설정 파일 탐색/병합 로더.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::json;

use crate::application::config::Config;
use crate::application::config::DEFAULT_SYSTEM_PROMPT;

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

    if let Ok(path) = env::var("REPOPILOT_CONFIG")
        && !Path::new(&path).exists()
    {
        bootstrap_template_bundle(Path::new(&path))?;
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
        // 최초 실행 경험을 위해 로컬 기본 설정 템플릿을 자동 생성한다.
        let bootstrap_target = default_bootstrap_config_path();
        bootstrap_template_bundle(&bootstrap_target)?;

        let raw = fs::read_to_string(&bootstrap_target).with_context(|| {
            format!(
                "failed to read bootstrapped config at {}",
                bootstrap_target.display()
            )
        })?;
        let parsed: Config = serde_json::from_str(&raw).with_context(|| {
            format!(
                "failed to parse bootstrapped JSON in {}",
                bootstrap_target.display()
            )
        })?;
        merged.merge_from(parsed);
        loaded_paths.push(bootstrap_target);
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
    let mut paths = vec![PathBuf::from("/etc/repopilot/config.json")];

    if let Some(base) = dirs::config_dir() {
        paths.push(base.join("repopilot").join("config.json"));
    }

    paths.push(PathBuf::from(".repopilot/config.json"));

    if let Ok(path) = env::var("REPOPILOT_CONFIG") {
        paths.push(Path::new(&path).to_path_buf());
    }

    dedup_paths(paths)
}

/// 편집 대상 설정 파일 경로를 결정한다.
/// 로딩된 파일 중 최고 우선순위 경로를 반환하고,
/// 로딩된 파일이 없으면 `.repopilot/config.json`을 생성한다.
pub(crate) fn editable_config_path() -> Result<PathBuf> {
    let loaded = load_merged_config();

    // 로딩 성공 시 최고 우선순위(마지막) 경로를 반환한다.
    if let Ok(lc) = loaded
        && let Some(last) = lc.loaded_paths.last()
    {
        return Ok(last.clone());
    }

    // 설정 파일이 없으면 프로젝트 로컬 기본 템플릿을 생성한다.
    let fallback = PathBuf::from(".repopilot/config.json");
    if let Some(parent) = fallback.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    fs::write(&fallback, "{}\n")
        .with_context(|| format!("failed to create default config at {}", fallback.display()))?;
    Ok(fallback)
}

fn default_bootstrap_config_path() -> PathBuf {
    if let Ok(path) = env::var("REPOPILOT_CONFIG") {
        return PathBuf::from(path);
    }
    PathBuf::from(".repopilot/config.json")
}

fn bootstrap_template_bundle(config_path: &Path) -> Result<()> {
    if config_path.exists() {
        return Ok(());
    }

    if let Some(parent) = config_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }

    let guide_path = default_review_guide_path(config_path);
    if !guide_path.exists() {
        if let Some(parent) = guide_path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {}", parent.display()))?;
        }
        fs::write(&guide_path, default_review_guide_template()).with_context(|| {
            format!(
                "failed to create review guide template at {}",
                guide_path.display()
            )
        })?;
    }

    let review_guide_path = if guide_path == Path::new("review-guide.md") {
        "./review-guide.md".to_string()
    } else {
        guide_path.display().to_string()
    };

    let template = json!({
        "defaults": {
            "max_diff_bytes": 120000,
            "system_prompt": DEFAULT_SYSTEM_PROMPT,
            "review_guide_path": review_guide_path,
            "comment_language": "ko",
            "update_timeout_ms": 1200
        },
        "hosts": {
            "github.com": {
                "token_env": "GITHUB_TOKEN",
                "token_command": ["gh", "auth", "token"]
            },
            "gitlab.com": {
                "token_env": "GITLAB_TOKEN",
                "token_command": ["glab", "auth", "token"]
            }
        },
        "providers": {
            "openai": {
                "enabled": true,
                "api_key_env": "OPENAI_API_KEY",
                "model": "gpt-4.1-mini",
                "command": "codex",
                "auto_auth": true,
                "auth_command": ["codex", "login"],
                "args": ["exec"]
            },
            "anthropic": {
                "enabled": true,
                "api_key_env": "ANTHROPIC_API_KEY",
                "model": "claude-3-7-sonnet-latest",
                "command": "claude",
                "auto_auth": true,
                "auth_command": ["claude", "login"],
                "args": []
            },
            "gemini": {
                "enabled": true,
                "api_key_env": "GEMINI_API_KEY",
                "model": "gemini-2.0-flash",
                "command": "gemini",
                "auto_auth": true,
                "auth_command": ["gemini", "login"],
                "args": []
            }
        }
    });

    let rendered = serde_json::to_string_pretty(&template)?;
    fs::write(config_path, format!("{rendered}\n"))
        .with_context(|| format!("failed to create config template at {}", config_path.display()))
}

fn default_review_guide_path(config_path: &Path) -> PathBuf {
    match config_path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join("review-guide.md"),
        _ => PathBuf::from("review-guide.md"),
    }
}

fn default_review_guide_template() -> &'static str {
    r#"# Review Guide

아래 원칙을 기준으로 Pull Request/Merge Request를 리뷰하세요.

## Output Format
- Critical
- Major
- Minor
- Suggestions

## Rules
- 근거가 불충분하면 추측하지 말고 확인 질문을 남긴다.
- 보안/데이터 손상/권한 문제는 우선적으로 보고한다.
- 재현 가능한 시나리오와 수정 제안을 함께 제공한다.
- 한국어로 간결하고 구체적으로 작성한다.
"#
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
