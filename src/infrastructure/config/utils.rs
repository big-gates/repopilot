//! 설정 모듈 공용 유틸리티.

use std::env;
use std::path::Path;

/// 로컬 명령이 실행 가능한지 탐지한다.
pub fn command_exists(command: &str) -> bool {
    // 절대/상대 경로가 주어지면 파일 존재만 검사한다.
    if command.trim().is_empty() {
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
    #[cfg(windows)]
    {
        // Windows는 확장자를 생략할 수 있으므로 PATHEXT를 고려한다.
        let has_ext = command_path.extension().is_some();
        let pathext = env::var_os("PATHEXT").unwrap_or_else(|| ".EXE;.CMD;.BAT;.COM".into());
        let exts: Vec<String> = pathext
            .to_string_lossy()
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        for dir in env::split_paths(&path_var) {
            if dir.join(command).is_file() {
                return true;
            }
            if !has_ext {
                for ext in &exts {
                    if dir.join(format!("{command}{ext}")).is_file() {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    #[cfg(not(windows))]
    {
        for dir in env::split_paths(&path_var) {
            if dir.join(command).is_file() {
                return true;
            }
        }
        false
    }
}
