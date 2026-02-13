//! 설정 파일 탐색/병합 결과를 확인하는 유스케이스.

use anyhow::Result;

use crate::application::ports::ConfigRepository;

/// 현재 적용 중인 설정을 사람이 읽기 쉬운 JSON으로 반환한다.
pub struct InspectConfigUseCase<'a> {
    pub config_repo: &'a dyn ConfigRepository,
}

impl<'a> InspectConfigUseCase<'a> {
    /// 설정 점검 결과 문자열을 생성한다.
    pub fn execute(&self) -> Result<String> {
        self.config_repo.inspect_pretty_json()
    }
}
