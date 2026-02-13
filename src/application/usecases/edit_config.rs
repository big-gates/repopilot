//! 설정 파일 편집 경로를 반환하는 유스케이스.

use std::path::PathBuf;

use anyhow::Result;

use crate::application::ports::ConfigRepository;

/// 편집 대상 설정 파일 경로를 조회한다.
pub struct EditConfigUseCase<'a> {
    pub config_repo: &'a dyn ConfigRepository,
}

impl<'a> EditConfigUseCase<'a> {
    /// 편집 대상 설정 파일 경로를 반환한다.
    pub fn execute(&self) -> Result<PathBuf> {
        self.config_repo.editable_config_path()
    }
}
