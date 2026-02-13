//! 애플리케이션 조립(composition root) 모듈.

use crate::application::usecases::inspect_config::InspectConfigUseCase;
use crate::application::usecases::review_pr::ReviewPrUseCase;
use crate::infrastructure::adapters::{
    ConsoleReporter, JsonConfigRepository, MarkdownRendererAdapter, ProviderFactoryAdapter,
    UrlTargetResolver, VcsFactoryAdapter,
};

/// 실행 시점 의존성을 한 곳에서 조립하는 컨테이너.
pub struct AppComposition {
    config_repo: JsonConfigRepository,
    target_resolver: UrlTargetResolver,
    vcs_factory: VcsFactoryAdapter,
    provider_factory: ProviderFactoryAdapter,
    renderer: MarkdownRendererAdapter,
    reporter: ConsoleReporter,
}

impl Default for AppComposition {
    fn default() -> Self {
        Self {
            config_repo: JsonConfigRepository,
            target_resolver: UrlTargetResolver,
            vcs_factory: VcsFactoryAdapter,
            provider_factory: ProviderFactoryAdapter,
            renderer: MarkdownRendererAdapter,
            reporter: ConsoleReporter,
        }
    }
}

impl AppComposition {
    /// 설정 점검 유스케이스를 생성한다.
    pub fn inspect_config_usecase(&self) -> InspectConfigUseCase<'_> {
        InspectConfigUseCase {
            config_repo: &self.config_repo,
        }
    }

    /// 리뷰 실행 유스케이스를 생성한다.
    pub fn review_usecase(&self) -> ReviewPrUseCase<'_> {
        ReviewPrUseCase {
            config_repo: &self.config_repo,
            target_resolver: &self.target_resolver,
            vcs_factory: &self.vcs_factory,
            provider_factory: &self.provider_factory,
            renderer: &self.renderer,
            reporter: &self.reporter,
        }
    }
}
