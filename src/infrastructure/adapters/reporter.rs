//! 콘솔 리포터 포트 구현 어댑터.

use crate::application::ports::Reporter;

/// 콘솔 전용 리포터 어댑터.
pub struct ConsoleReporter;

impl Reporter for ConsoleReporter {
    fn section(&self, name: &str) {
        println!();
        println!("==================== {} ====================", name);
    }

    fn kv(&self, key: &str, value: &str) {
        println!("{:<12}: {}", key, value);
    }

    fn status(&self, scope: &str, message: &str) {
        println!("[{:<12}] {}", scope, message);
    }

    fn provider_status(&self, provider: &str, status: &str, extra: Option<&str>) {
        match extra {
            Some(extra) => println!("[provider:{:<12}] {:<7} {}", provider, status, extra),
            None => println!("[provider:{:<12}] {}", provider, status),
        }
    }

    fn raw(&self, line: &str) {
        println!("{}", line);
    }
}
