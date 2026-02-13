//! `prpilot` 바이너리 진입점.

use prpilot::interface::cli::{AppComposition, Cli, CliAction, run_repl};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn".into()),
        )
        .init();

    let action = match Cli::parse_action() {
        Ok(action) => action,
        Err(msg) => {
            eprintln!("error: {msg}");
            std::process::exit(2);
        }
    };

    match action {
        CliAction::Interactive => {
            // REPL 하단 UI와 충돌하지 않도록 provider 상태판은 끈다.
            let composition = AppComposition::new(false);
            if let Err(err) = run_repl(&composition).await {
                eprintln!("error: {err:#}");
                std::process::exit(1);
            }
        }
        CliAction::InspectConfig => {
            let composition = AppComposition::default();
            match composition.inspect_config_usecase().execute() {
                Ok(json) => println!("{json}"),
                Err(err) => {
                    eprintln!("error: {err:#}");
                    std::process::exit(1);
                }
            }
        }
        CliAction::Review(options) => {
            let composition = AppComposition::default();
            if let Err(err) = composition.review_usecase().execute(options).await {
                eprintln!("error: {err:#}");
                std::process::exit(1);
            }
        }
    }
}
