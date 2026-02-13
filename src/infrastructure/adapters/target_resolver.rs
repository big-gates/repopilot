//! 대상 URL 파싱 포트 구현 어댑터.

use anyhow::Result;

use crate::application::ports::TargetResolver;
use crate::domain::target::ReviewTarget;

/// URL 문자열을 도메인 타깃으로 변환하는 어댑터.
pub struct UrlTargetResolver;

impl TargetResolver for UrlTargetResolver {
    fn parse(&self, input: &str) -> Result<ReviewTarget> {
        ReviewTarget::parse(input)
    }
}
