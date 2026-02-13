//! 콘솔 리포터 포트 구현 어댑터.

use std::collections::BTreeMap;
use std::io::{self, IsTerminal, Write};
use std::sync::Mutex;

use crate::application::ports::Reporter;

#[derive(Default)]
struct ProviderPanelState {
    in_provider_section: bool,
    rendered_lines: usize,
    rows: BTreeMap<String, (String, Option<String>)>,
}

/// 콘솔 전용 리포터 어댑터.
pub struct ConsoleReporter {
    interactive: bool,
    provider_panel_enabled: bool,
    state: Mutex<ProviderPanelState>,
}

impl Default for ConsoleReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleReporter {
    /// stdout이 TTY일 때 실시간 상태판 모드를 활성화한다.
    pub fn new() -> Self {
        Self::with_provider_panel(true)
    }

    /// REPL UI와 충돌을 피해야 할 때 provider 상태판을 비활성화할 수 있다.
    pub fn with_provider_panel(enabled: bool) -> Self {
        Self {
            interactive: io::stdout().is_terminal(),
            provider_panel_enabled: enabled,
            state: Mutex::new(ProviderPanelState::default()),
        }
    }

    fn set_section(&self, name: &str) {
        if !self.interactive {
            return;
        }

        if let Ok(mut state) = self.state.lock() {
            state.in_provider_section = name.starts_with("Providers (");
            state.rows.clear();
            state.rendered_lines = 0;
        }
    }

    fn render_provider_panel(&self, state: &mut ProviderPanelState) {
        let mut out = io::stdout();
        if state.rendered_lines > 0 {
            let _ = write!(out, "\x1b[{}A\x1b[J", state.rendered_lines);
        }

        let mut lines = Vec::new();
        lines.push("┌──────────────── Provider Status ────────────────┐".to_string());
        for (provider, (status, extra)) in &state.rows {
            let status_colored = colorize_status(status);
            let extra_text = extra.as_deref().unwrap_or("-");
            lines.push(format!(
                "│ {:<14} {:<16} {:<18} │",
                provider, status_colored, extra_text
            ));
        }
        lines.push("└──────────────────────────────────────────────────┘".to_string());

        for line in &lines {
            let _ = writeln!(out, "{line}");
        }
        let _ = out.flush();
        state.rendered_lines = lines.len();
    }
}

impl Reporter for ConsoleReporter {
    fn section(&self, name: &str) {
        self.set_section(name);
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
        if self.interactive
            && self.provider_panel_enabled
            && let Ok(mut state) = self.state.lock()
            && state.in_provider_section
        {
            state.rows.insert(
                provider.to_string(),
                (status.to_string(), extra.map(|s| s.to_string())),
            );
            self.render_provider_panel(&mut state);
            return;
        }

        match extra {
            Some(extra) => println!("[provider:{:<12}] {:<7} {}", provider, status, extra),
            None => println!("[provider:{:<12}] {}", provider, status),
        }
    }

    fn raw(&self, line: &str) {
        println!("{}", line);
    }
}

fn colorize_status(status: &str) -> String {
    match status {
        "running" => format!("\x1b[33m{status}\x1b[0m"),
        "done" => format!("\x1b[32m{status}\x1b[0m"),
        "error" => format!("\x1b[31m{status}\x1b[0m"),
        _ => status.to_string(),
    }
}
