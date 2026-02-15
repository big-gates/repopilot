//! VCS OAuth 인증 포트 구현(gh/glab).

use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

use crate::application::ports::{VcsAuthKind, VcsAuthenticator};
use crate::infrastructure::config::command_exists;

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
    if !command_exists("gh") {
        bail!(
            "GitHub CLI (`gh`) not found in PATH.\n\
Install it first, then re-run `repopilot auth github`.\n\
- macOS: brew install gh\n\
- Windows: winget install --id GitHub.cli\n\
- Linux: install `gh` via your package manager"
        );
    }

    let mut cmd = Command::new("gh");
    cmd.args(["auth", "login"]);
    if host != "github.com" {
        cmd.args(["--hostname", host]);
    }

    let status = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| "failed to run `gh auth login` (install GitHub CLI: gh)")?;
    if !status.success() {
        bail!("`gh auth login` exited with {status}");
    }
    Ok(())
}

fn glab_auth_login(host: &str) -> Result<()> {
    if !command_exists("glab") {
        bail!(
            "GitLab CLI (`glab`) not found in PATH.\n\
Install it first, then re-run `repopilot auth gitlab`.\n\
- macOS: brew install glab\n\
- Windows: winget install --id glab.glab\n\
- Linux: install `glab` via your package manager"
        );
    }

    let mut cmd = Command::new("glab");
    cmd.args(["auth", "login"]);
    if host != "gitlab.com" {
        cmd.args(["--hostname", host]);
    }

    let status = cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| "failed to run `glab auth login` (install GitLab CLI: glab)")?;
    if !status.success() {
        bail!("`glab auth login` exited with {status}");
    }
    Ok(())
}
