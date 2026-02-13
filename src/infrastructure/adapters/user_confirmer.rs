//! 사용자 확인 입력 포트 구현 어댑터.

use std::io::{self, Write};

use anyhow::Result;

use crate::application::ports::UserConfirmer;

/// stdin으로 yes/y 확인을 받는 어댑터.
pub struct StdinConfirmer;

impl UserConfirmer for StdinConfirmer {
    fn confirm(&self, message: &str) -> Result<bool> {
        eprintln!("{message}");
        eprint!("continue? (y/yes): ");
        io::stderr().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let answer = input.trim().to_ascii_lowercase();

        Ok(answer == "y" || answer == "yes")
    }
}

/// 항상 승인하는 무조건 확인 어댑터(라이브러리 직접 호출용).
pub struct AutoConfirmer;

impl UserConfirmer for AutoConfirmer {
    fn confirm(&self, _message: &str) -> Result<bool> {
        Ok(true)
    }
}
