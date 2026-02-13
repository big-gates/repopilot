//! `prpilot` 대화형 쉘(REPL) 인터페이스.

use std::io::{self, IsTerminal, Write};

use anyhow::Result;

use crate::domain::review::RunOptions;
use crate::interface::cli::composition::AppComposition;
use crate::interface::cli::repl_input::read_repl_input;

/// 대화형 입력으로 `/command`를 처리한다.
pub async fn run_repl(composition: &AppComposition) -> Result<()> {
    print_welcome();
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
            if parts.len() != 1 {
                return Err("usage: /config".to_string());
            }
            Ok(ReplCommand::InspectConfig)
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

fn print_welcome() {
    let interactive = io::stdout().is_terminal();
    if interactive {
        // 대화형 터미널에서는 시작 화면을 지우고 배너를 출력한다.
        print!("\x1b[2J\x1b[H");
    }

    let title = paint("prpilot interactive shell", "1;36", interactive);
    let subtitle = paint("multi-agent review cockpit", "2;37", interactive);
    let cmd_palette = paint("/", "1;33", interactive);
    let cmd_config = paint("/config", "1;32", interactive);
    let cmd_review = paint("/review <url> [--dry-run] [--force]", "1;35", interactive);
    let cmd_exit = paint("/exit", "1;31", interactive);

    println!("+------------------------------------------------------------+");
    println!("| {:<58} |", title);
    println!("| {:<58} |", subtitle);
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
