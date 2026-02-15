//! VCS OAuth 인증(로그인) 유스케이스.

use anyhow::Result;

use crate::application::ports::{VcsAuthKind, VcsAuthenticator};

/// gh/glab 등 OAuth 로그인을 수행한다.
pub struct AuthVcsUseCase<'a> {
    pub authenticator: &'a dyn VcsAuthenticator,
}

impl<'a> AuthVcsUseCase<'a> {
    pub fn execute(&self, kind: VcsAuthKind, host: &str) -> Result<()> {
        self.authenticator.authenticate(kind, host)
    }
}

