//! 애플리케이션 포트를 실제 인프라 구현체로 연결하는 어댑터 계층.

mod config_repository;
mod markdown_renderer;
mod provider_factory;
mod reporter;
mod target_resolver;
mod update_checker;
mod user_confirmer;
mod vcs_factory;

pub use config_repository::JsonConfigRepository;
pub use markdown_renderer::MarkdownRendererAdapter;
pub use provider_factory::ProviderFactoryAdapter;
pub use reporter::ConsoleReporter;
pub use target_resolver::UrlTargetResolver;
pub use update_checker::HttpUpdateChecker;
pub use user_confirmer::{AutoConfirmer, StdinConfirmer};
pub use vcs_factory::VcsFactoryAdapter;
