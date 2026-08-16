use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Bumped on any wire-format or host-behaviour change; a mismatch in `Pong`
/// tells the app to reinstall the service.
pub const PROTOCOL_VERSION: u32 = 2;

/// Everything the host needs to launch a core — the app resolves every path.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CoreSpec {
    pub binary: PathBuf,
    pub work_dir: PathBuf,
    pub config: PathBuf,
    #[serde(default = "default_max_log_days")]
    pub max_log_days: u32,
}

fn default_max_log_days() -> u32 {
    7
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "action", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Request {
    StartCore(CoreSpec),
    StopCore,
    Ping,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Response {
    Started {
        pid: u32,
    },
    Ok,
    Pong {
        protocol_version: u32,
        core_pid: Option<u32>,
    },
    Error {
        message: String,
    },
}
