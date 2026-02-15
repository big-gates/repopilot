//! 시스템 프롬프트 해석 포트 구현(리뷰 가이드 파일 포함).

use std::fs;

use anyhow::{Context, Result};

use crate::application::config::Config;
use crate::application::ports::SystemPromptResolver;

/// 설정의 `review_guide_path`를 읽어 시스템 프롬프트에 합성한다.
pub struct FileSystemPromptResolver;

impl SystemPromptResolver for FileSystemPromptResolver {
    fn resolve(&self, config: &Config) -> Result<String> {
        let mut prompt = config.system_prompt();

        let Some(path) = config.defaults.review_guide_path.as_deref() else {
            return Ok(prompt);
        };

        let guide_raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read review guide file at {}", path))?;
        let guide = guide_raw.trim();
        if guide.is_empty() {
            return Ok(prompt);
        }

        prompt.push_str("\n\nReview guide (must follow):\n");
        prompt.push_str(guide);
        Ok(prompt)
    }
}

