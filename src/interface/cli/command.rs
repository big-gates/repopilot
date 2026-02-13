//! CLI 명령 파싱 모듈.

use clap::{Parser, Subcommand};

use crate::domain::review::RunOptions;

#[derive(Debug, Parser)]
#[command(name = "prpilot")]
#[command(about = "Multi-agent review for GitHub PRs and GitLab MRs")]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// PR/MR URL
    url: Option<String>,

    /// Print markdown to stdout, do not post
    #[arg(long)]
    dry_run: bool,

    /// Re-run even if current SHA is already claimed/reviewed
    #[arg(long)]
    force: bool,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Show effective merged config and provider command availability
    Config,
}

pub enum CliAction {
    Interactive,
    InspectConfig,
    Review(RunOptions),
}

impl Cli {
    pub fn parse_action() -> Result<CliAction, String> {
        let cli = Cli::parse();

        match cli.command {
            Some(Commands::Config) => Ok(CliAction::InspectConfig),
            None => {
                let Some(url) = cli.url else {
                    return Ok(CliAction::Interactive);
                };

                Ok(CliAction::Review(RunOptions {
                    url,
                    dry_run: cli.dry_run,
                    force: cli.force,
                }))
            }
        }
    }
}
