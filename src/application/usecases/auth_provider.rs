//! Provider OAuth 인증(로그인) 유스케이스.

use anyhow::Result;

use crate::application::ports::{ConfigRepository, ProviderAuthKind, ProviderAuthenticator};

/// codex/claude/gemini 로그인(OAuth)을 수행한다.
pub struct AuthProviderUseCase<'a> {
    pub config_repo: &'a dyn ConfigRepository,
    pub authenticator: &'a dyn ProviderAuthenticator,
}

impl<'a> AuthProviderUseCase<'a> {
    pub fn execute(&self, kind: ProviderAuthKind) -> Result<()> {
        // config는 auth_command(사용자 커스텀) 조회 용도로만 사용한다.
        let cfg = self.config_repo.load()?;
        let provider_cfg = match kind {
            ProviderAuthKind::Codex => cfg.providers.openai.as_ref(),
            ProviderAuthKind::Claude => cfg.providers.anthropic.as_ref(),
            ProviderAuthKind::Gemini => cfg.providers.gemini.as_ref(),
        };

        self.authenticator.authenticate(kind, provider_cfg)
    }
}

