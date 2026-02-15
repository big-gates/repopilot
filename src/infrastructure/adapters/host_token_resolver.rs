//! 호스트(VCS) 토큰 해석 포트 구현.

use anyhow::Result;

use crate::application::config::HostConfig;
use crate::application::ports::{HostTokenResolution, HostTokenResolver};
use crate::infrastructure::config::resolve_host_token;

/// 설정(token/env/cmd)에 기반해 런타임 토큰을 해석한다.
pub struct HostTokenResolverAdapter;

impl HostTokenResolver for HostTokenResolverAdapter {
    fn resolve(&self, _host: &str, host_cfg: Option<&HostConfig>) -> Result<HostTokenResolution> {
        resolve_host_token(host_cfg)
    }
}

