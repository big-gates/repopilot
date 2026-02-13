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

    // 시작 시 최신 버전 알림을 시도한다(실패 시 무시).
    let update_composition = AppComposition::default();
    if let Ok(Some(notice)) = update_composition.check_update_usecase().execute().await {
        eprintln!(
            "update available: {} -> {}",
            notice.current_version, notice.latest_version
        );
        if let Some(url) = notice.download_url {
            eprintln!("update url: {url}");
        }
    }

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
