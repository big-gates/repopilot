//! CLI 명령 파싱 모듈.

use clap::{Parser, Subcommand};

use crate::application::ports::{ProviderAuthKind, VcsAuthKind};
use crate::domain::review::RunOptions;

#[derive(Debug, Parser)]
#[command(name = "repopilot")]
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
    /// OAuth login via VCS/provider CLI
    Auth {
        #[command(subcommand)]
        provider: AuthProvider,
    },
}

#[derive(Debug, Subcommand)]
enum AuthProvider {
    /// GitHub OAuth login (`gh auth login`)
    Github {
        /// GitHub hostname (default: github.com)
        #[arg(long, default_value = "github.com")]
        host: String,
    },
    /// GitLab OAuth login (`glab auth login`)
    Gitlab {
        /// GitLab hostname (default: gitlab.com)
        #[arg(long, default_value = "gitlab.com")]
        host: String,
    },
    /// OpenAI/Codex OAuth login (provider CLI)
    Codex,
    /// Anthropic/Claude OAuth login (provider CLI)
    Claude,
    /// Google Gemini OAuth login (provider CLI)
    Gemini,
}

pub enum CliAction {
    Interactive,
    InspectConfig,
    Review(RunOptions),
    Auth { kind: VcsAuthKind, host: String },
    AuthProvider { kind: ProviderAuthKind },
}

impl Cli {
    pub fn parse_action() -> Result<CliAction, String> {
        let cli = Cli::parse();

        match cli.command {
            Some(Commands::Config) => Ok(CliAction::InspectConfig),
            Some(Commands::Auth { provider }) => match provider {
                AuthProvider::Github { host } => Ok(CliAction::Auth {
                    kind: VcsAuthKind::GitHub,
                    host,
                }),
                AuthProvider::Gitlab { host } => Ok(CliAction::Auth {
                    kind: VcsAuthKind::GitLab,
                    host,
                }),
                AuthProvider::Codex => Ok(CliAction::AuthProvider {
                    kind: ProviderAuthKind::Codex,
                }),
                AuthProvider::Claude => Ok(CliAction::AuthProvider {
                    kind: ProviderAuthKind::Claude,
                }),
                AuthProvider::Gemini => Ok(CliAction::AuthProvider {
                    kind: ProviderAuthKind::Gemini,
                }),
            },
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
