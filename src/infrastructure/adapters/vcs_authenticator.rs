//! VCS OAuth 인증 포트 구현(gh/glab).

use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::application::ports::{VcsAuthKind, VcsAuthenticator};

/// 외부 CLI를 이용해 OAuth 로그인을 수행한다.
pub struct VcsAuthenticatorAdapter;

impl VcsAuthenticator for VcsAuthenticatorAdapter {
    fn authenticate(&self, kind: VcsAuthKind, host: &str) -> Result<()> {
        match kind {
            VcsAuthKind::GitHub => gh_auth_login(host),
            VcsAuthKind::GitLab => glab_auth_login(host),
        }
    }
}

fn gh_auth_login(host: &str) -> Result<()> {
    let mut cmd = Command::new("gh");
    cmd.args(["auth", "login"]);
    if host != "github.com" {
        cmd.args(["--hostname", host]);
    }

    let status = cmd
        .status()
        .with_context(|| "failed to run `gh auth login` (install GitHub CLI: gh)")?;
    if !status.success() {
        bail!("`gh auth login` exited with {status}");
    }
    Ok(())
}

fn glab_auth_login(host: &str) -> Result<()> {
    let mut cmd = Command::new("glab");
    cmd.args(["auth", "login"]);
    if host != "gitlab.com" {
        cmd.args(["--hostname", host]);
    }

    let status = cmd
        .status()
        .with_context(|| "failed to run `glab auth login` (install GitLab CLI: glab)")?;
    if !status.success() {
        bail!("`glab auth login` exited with {status}");
    }
    Ok(())
}

