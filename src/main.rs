//! `prpilot` 바이너리 진입점.

use prpilot::interface::cli::{Cli, CliAction};
use prpilot::interface::composition::AppComposition;

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

    let composition = AppComposition::default();

    match action {
        CliAction::InspectConfig => {
            match composition.inspect_config_usecase().execute() {
                Ok(json) => println!("{json}"),
                Err(err) => {
                    eprintln!("error: {err:#}");
                    std::process::exit(1);
                }
            }
        }
        CliAction::Review(options) => {
            if let Err(err) = composition.review_usecase().execute(options).await {
                eprintln!("error: {err:#}");
                std::process::exit(1);
            }
        }
    }
}
