//! `RepoPilot` 대화형 쉘(REPL) 인터페이스.

use std::io::{self, IsTerminal, Write};
use std::process::Command;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::domain::review::RunOptions;
use crate::interface::cli::composition::AppComposition;
use crate::interface::cli::repl_input::read_repl_input;

/// 대화형 입력으로 `/command`를 처리한다.
pub async fn run_repl(composition: &AppComposition) -> Result<()> {
    print_welcome(composition);
    io::stdout().flush()?;
    let mut next_prefill: Option<String> = None;

    loop {
        let prefill = next_prefill.take();
        let Some(raw_input) = read_repl_input(prefill.as_deref())? else {
            println!();
            break;
        };
        let input = raw_input.trim();
        if input.is_empty() {
            continue;
        }

        match parse_repl_command(input) {
            Ok(ReplCommand::Exit) => break,
            Ok(ReplCommand::ReviewNeedsArgs) => {
                // 인자가 빠진 `/review`는 별도 프롬프트를 띄우지 않고 입력창에 재프리필한다.
                next_prefill = Some("/review ".to_string());
            }
            Ok(cmd) => {
                if let Err(err) = execute_command(composition, cmd).await {
                    eprintln!("error: {err:#}");
                }
            }
            Err(msg) => {
                eprintln!("error: {msg}");
                eprintln!("hint: use start typing / for command suggestions");
            }
        }
    }

    Ok(())
}

enum ReplCommand {
    Exit,
    InspectConfig,
    EditConfig,
    /// `/review`만 입력된 상태. 다음 입력 라운드에 `/review `를 프리필한다.
    ReviewNeedsArgs,
    Review(RunOptions),
}

async fn execute_command(composition: &AppComposition, command: ReplCommand) -> Result<()> {
    match command {
        ReplCommand::Exit => Ok(()),
        ReplCommand::InspectConfig => {
            let json = composition.inspect_config_usecase().execute()?;
            println!("{json}");
            Ok(())
        }
        ReplCommand::EditConfig => {
            let path = composition.edit_config_usecase().execute()?;
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

            // 에디터가 정상 동작하도록 raw mode를 해제한다.
            let _ = crossterm::terminal::disable_raw_mode();
            let status = Command::new(&editor)
                .arg(&path)
                .status()
                .with_context(|| format!("failed to launch editor: {editor}"))?;
            let _ = crossterm::terminal::enable_raw_mode();

            if status.success() {
                println!("config saved: {}", path.display());
            } else {
                eprintln!("editor exited with: {status}");
            }
            Ok(())
        }
        ReplCommand::ReviewNeedsArgs => Ok(()),
        ReplCommand::Review(options) => {
            composition.review_usecase().execute(options).await?;
            Ok(())
        }
    }
}

fn parse_repl_command(input: &str) -> Result<ReplCommand, String> {
    if !input.starts_with('/') {
        return Err("slash command only. example: /review <url>".to_string());
    }

    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return Err("empty command".to_string());
    }

    match parts[0] {
        "/exit" | "/quit" => Ok(ReplCommand::Exit),
        "/config" => {
            if parts.len() == 1 {
                return Ok(ReplCommand::InspectConfig);
            }
            if parts.len() == 2 && parts[1] == "edit" {
                return Ok(ReplCommand::EditConfig);
            }
            Err("usage: /config [edit]".to_string())
        }
        "/review" => {
            if parts.len() == 1 {
                Ok(ReplCommand::ReviewNeedsArgs)
            } else {
                parse_review_command(&parts[1..]).map(ReplCommand::Review)
            }
        }
        other => Err(format!("unknown command: {other}")),
    }
}

fn parse_review_command(args: &[&str]) -> Result<RunOptions, String> {
    if args.is_empty() {
        return Err("usage: /review <url> [--dry-run] [--force]".to_string());
    }

    let mut url: Option<String> = None;
    let mut dry_run = false;
    let mut force = false;

    for arg in args {
        match *arg {
            "--dry-run" => dry_run = true,
            "--force" => force = true,
            _ if arg.starts_with("--") => {
                return Err(format!("unknown option: {arg}"));
            }
            _ => {
                if url.is_some() {
                    return Err(
                        "usage: /review <url> [--dry-run] [--force] (url must be single)"
                            .to_string(),
                    );
                }
                url = Some((*arg).to_string());
            }
        }
    }

    let Some(url) = url else {
        return Err("usage: /review <url> [--dry-run] [--force]".to_string());
    };

    Ok(RunOptions {
        url,
        dry_run,
        force,
    })
}

fn print_welcome(composition: &AppComposition) {
    let interactive = io::stdout().is_terminal();
    if interactive {
        // 대화형 터미널에서는 시작 화면을 지우고 배너를 출력한다.
        print!("\x1b[2J\x1b[H");
    }

    let title = paint("RepoPilot interactive shell", "1;36", interactive);
    let subtitle = paint("multi-agent review cockpit", "2;37", interactive);
    let cmd_palette = paint("/", "1;33", interactive);
    let cmd_config = paint("/config [edit]", "1;32", interactive);
    let cmd_review = paint("/review <url> [--dry-run] [--force]", "1;35", interactive);
    let cmd_exit = paint("/exit", "1;31", interactive);

    println!("+------------------------------------------------------------+");
    println!("| {:<58} |", title);
    println!("| {:<58} |", subtitle);
    println!("+------------------------------------------------------------+");
    println!("| Status Dashboard                                            |");
    for line in build_startup_dashboard_lines(composition) {
        println!("| {:<58} |", fit_box_line(&line, 58));
    }
    println!("+------------------------------------------------------------+");
    println!("| Quick start                                                 |");
    println!("|  0) {:<54} |", cmd_palette);
    println!("|  1) {:<54} |", cmd_config);
    println!("|  2) {:<54} |", cmd_review);
    println!("|  3) {:<54} |", cmd_exit);
    println!("+------------------------------------------------------------+");
    println!();
}

fn paint(text: &str, ansi: &str, interactive: bool) -> String {
    if interactive {
        format!("\x1b[{ansi}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

fn build_startup_dashboard_lines(composition: &AppComposition) -> Vec<String> {
    let mut lines = Vec::new();

    let inspection_json = match composition.inspect_config_usecase().execute() {
        Ok(raw) => raw,
        Err(err) => {
            lines.push("Config: error".to_string());
            lines.push(format!("detail: {err}"));
            lines.push("hint: run `/config` to inspect and fix".to_string());
            return lines;
        }
    };

    let value: Value = match serde_json::from_str(&inspection_json) {
        Ok(v) => v,
        Err(_) => {
            lines.push("Config: loaded (dashboard parse fallback)".to_string());
            lines.push("hint: run `/config` to inspect details".to_string());
            return lines;
        }
    };

    let loaded_count = value
        .get("loaded_paths")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);
    lines.push(format!("Config: ok (loaded files: {loaded_count})"));

    let guide = value
        .pointer("/effective_defaults/review_guide_path")
        .and_then(|v| v.as_str())
        .unwrap_or("not set");
    let lang = value
        .pointer("/effective_defaults/comment_language")
        .and_then(|v| v.as_str())
        .unwrap_or("ko");
    lines.push(format!("Review Guide: {guide}"));
    lines.push(format!("Comment Language: {lang}"));

    if let Some(hosts) = value.get("hosts").and_then(|v| v.as_object()) {
        if hosts.is_empty() {
            lines.push("Hosts: not configured".to_string());
        } else {
            for (host, cfg) in hosts {
                let resolved = cfg
                    .get("token_resolved")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let token = if resolved { "resolved" } else { "missing" };
                lines.push(format!("Host: {host} (token {token})"));
            }
        }
    } else {
        lines.push("Hosts: unavailable".to_string());
    }

    if let Some(providers) = value.get("providers").and_then(|v| v.as_object()) {
        lines.push("Providers:".to_string());
        for key in ["openai", "anthropic", "gemini"] {
            let Some(cfg) = providers.get(key) else {
                lines.push(format!("  - {key:<10} not configured"));
                continue;
            };
            if cfg.is_null() {
                lines.push(format!("  - {key:<10} not configured"));
                continue;
            }

            let enabled = cfg
                .get("enabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let resolved_mode = cfg
                .get("resolved_mode")
                .and_then(|v| v.as_str())
                .unwrap_or("cli");
            let command = cfg.get("command").and_then(|v| v.as_str()).unwrap_or("-");
            let available = cfg
                .get("command_available")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let auth_status = cfg
                .get("auth_status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let runnable = cfg
                .get("runnable")
                .and_then(|v| v.as_bool())
                .unwrap_or(available);
            let state = if enabled { "enabled" } else { "disabled" };
            if resolved_mode == "api" {
                let run_status = if runnable { "ok" } else { "missing" };
                lines.push(format!("  - {key:<10} {state:<8} api auth=ok ({run_status})"));
            } else if !available {
                lines.push(format!("  - {key:<10} {state:<8} cli {command} cmd=missing"));
            } else {
                lines.push(format!(
                    "  - {key:<10} {state:<8} cli {command} auth={auth_status}"
                ));
                if auth_status != "ok"
                    && let Some(hint) = cfg.get("auth_hint").and_then(|v| v.as_str())
                {
                    lines.push(format!("    {hint}"));
                }
            }
        }
    }

    lines
}

fn fit_box_line(text: &str, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= width {
        return text.to_string();
    }

    if width <= 3 {
        return ".".repeat(width);
    }

    let keep = width - 3;
    let head: String = chars.into_iter().take(keep).collect();
    format!("{head}...")
}
