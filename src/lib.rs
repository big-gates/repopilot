//! prpilot library root.
//! Clean Architecture + DDD 계층을 외부에 노출한다.

use anyhow::Result;

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;

use domain::review::RunOptions;
use interface::cli::AppComposition;

/// 라이브러리 직접 호출용 실행 함수.
pub async fn run(options: RunOptions) -> Result<()> {
    let composition = AppComposition::default();
    composition.review_usecase().execute(options).await
}

/// 설정 점검 JSON 출력용 함수.
pub fn inspect_config_pretty_json() -> Result<String> {
    let composition = AppComposition::default();
    composition.inspect_config_usecase().execute()
}
