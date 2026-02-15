//! 애플리케이션 조립(composition root) 모듈.

use crate::application::ports::UserConfirmer;
use crate::application::usecases::auth_vcs::AuthVcsUseCase;
use crate::application::usecases::check_update::CheckUpdateUseCase;
use crate::application::usecases::edit_config::EditConfigUseCase;
use crate::application::usecases::inspect_config::InspectConfigUseCase;
use crate::application::usecases::review_pr::ReviewPrUseCase;
use crate::application::usecases::auth_provider::AuthProviderUseCase;
use crate::infrastructure::adapters::{
    ConsoleReporter, FileSystemPromptResolver, HostTokenResolverAdapter, HttpUpdateChecker,
    JsonConfigRepository, MarkdownRendererAdapter, ProviderFactoryAdapter, StdinConfirmer,
    ProviderAuthenticatorAdapter, UrlTargetResolver, VcsAuthenticatorAdapter, VcsFactoryAdapter,
};

/// 실행 시점 의존성을 한 곳에서 조립하는 컨테이너.
pub struct AppComposition {
    config_repo: JsonConfigRepository,
    host_token_resolver: HostTokenResolverAdapter,
    system_prompt_resolver: FileSystemPromptResolver,
    target_resolver: UrlTargetResolver,
    vcs_authenticator: VcsAuthenticatorAdapter,
    provider_authenticator: ProviderAuthenticatorAdapter,
    vcs_factory: VcsFactoryAdapter,
    provider_factory: ProviderFactoryAdapter,
    renderer: MarkdownRendererAdapter,
    reporter: ConsoleReporter,
    update_checker: HttpUpdateChecker,
    confirmer: Box<dyn UserConfirmer>,
}

impl Default for AppComposition {
    fn default() -> Self {
        Self::new(true)
    }
}

impl AppComposition {
    /// provider 상태판 사용 여부를 받아 실행 조합을 생성한다.
    pub fn new(provider_panel_enabled: bool) -> Self {
        Self::with_confirmer(provider_panel_enabled, Box::new(StdinConfirmer))
    }

    /// 확인 어댑터를 외부에서 주입한다.
    pub fn with_confirmer(
        provider_panel_enabled: bool,
        confirmer: Box<dyn UserConfirmer>,
    ) -> Self {
        Self {
            config_repo: JsonConfigRepository,
            host_token_resolver: HostTokenResolverAdapter,
            system_prompt_resolver: FileSystemPromptResolver,
            target_resolver: UrlTargetResolver,
            vcs_authenticator: VcsAuthenticatorAdapter,
            provider_authenticator: ProviderAuthenticatorAdapter,
            vcs_factory: VcsFactoryAdapter,
            provider_factory: ProviderFactoryAdapter,
            renderer: MarkdownRendererAdapter,
            reporter: ConsoleReporter::with_provider_panel(provider_panel_enabled),
            update_checker: HttpUpdateChecker,
            confirmer,
        }
    }

    /// 최신 버전 알림 유스케이스를 생성한다.
    pub fn check_update_usecase(&self) -> CheckUpdateUseCase<'_> {
        CheckUpdateUseCase {
            config_repo: &self.config_repo,
            host_token_resolver: &self.host_token_resolver,
            update_checker: &self.update_checker,
        }
    }

    /// VCS OAuth 인증 유스케이스를 생성한다.
    pub fn auth_vcs_usecase(&self) -> AuthVcsUseCase<'_> {
        AuthVcsUseCase {
            authenticator: &self.vcs_authenticator,
        }
    }

    /// Provider OAuth 인증 유스케이스를 생성한다.
    pub fn auth_provider_usecase(&self) -> AuthProviderUseCase<'_> {
        AuthProviderUseCase {
            config_repo: &self.config_repo,
            authenticator: &self.provider_authenticator,
        }
    }

    /// 설정 편집 유스케이스를 생성한다.
    pub fn edit_config_usecase(&self) -> EditConfigUseCase<'_> {
        EditConfigUseCase {
            config_repo: &self.config_repo,
        }
    }

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
            host_token_resolver: &self.host_token_resolver,
            system_prompt_resolver: &self.system_prompt_resolver,
            target_resolver: &self.target_resolver,
            vcs_factory: &self.vcs_factory,
            provider_factory: &self.provider_factory,
            renderer: &self.renderer,
            reporter: &self.reporter,
            confirmer: self.confirmer.as_ref(),
        }
    }
}
