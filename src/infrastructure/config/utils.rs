//! 설정 모듈 공용 유틸리티.

use std::env;
use std::path::Path;

/// 로컬 명령이 실행 가능한지 탐지한다.
pub fn command_exists(command: &str) -> bool {
    // 절대/상대 경로가 주어지면 파일 존재만 검사한다.
    if command.is_empty() {
        return false;
    }

    let command_path = Path::new(command);
    if command_path.components().count() > 1 {
        return command_path.is_file();
    }

    let Some(path_var) = env::var_os("PATH") else {
        return false;
    };

    // 일반 명령은 PATH를 순회해 탐지한다.
    for dir in env::split_paths(&path_var) {
        if dir.join(command).is_file() {
            return true;
        }
    }

    false
}
