#[cfg(target_os = "linux")]
pub(crate) mod linux;
#[cfg(windows)]
pub(crate) mod windows;

#[cfg(target_os = "linux")]
use linux as imp;
#[cfg(windows)]
use windows as imp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    NotInstalled,
    Stopped,
    Running,
    /// Installed, but pointing at a binary or protocol we can no longer talk to.
    Stale {
        reason: String,
    },
}

impl Status {
    pub fn is_running(&self) -> bool {
        matches!(self, Status::Running)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Status::NotInstalled => "not-installed",
            Status::Stopped => "stopped",
            Status::Running => "running",
            Status::Stale { .. } => "stale",
        }
    }
}

#[cfg(any(target_os = "linux", windows))]
pub use imp::{
    install, ping, restart_service, start_core, start_service, status, stop_core, stop_service,
    uninstall,
};

/// Whether the unit is owned by the OS rather than by Nyx.
#[cfg(target_os = "linux")]
pub fn is_managed() -> bool {
    linux::is_managed()
}

#[cfg(not(target_os = "linux"))]
pub fn is_managed() -> bool {
    false
}

#[cfg(not(any(target_os = "linux", windows)))]
mod unsupported {
    use super::Status;
    use crate::protocol::CoreSpec;

    pub async fn status() -> Result<Status, String> {
        Ok(Status::NotInstalled)
    }
    pub async fn install() -> Result<(), String> {
        Err("service mode is not supported on this platform".into())
    }
    pub async fn uninstall() -> Result<(), String> {
        Ok(())
    }
    pub async fn start_core(_spec: &CoreSpec) -> Result<u32, String> {
        Err("service mode is not supported on this platform".into())
    }
    pub async fn stop_core() -> Result<(), String> {
        Ok(())
    }
    pub async fn ping() -> Result<Option<u32>, String> {
        Err("service mode is not supported on this platform".into())
    }
    pub async fn start_service() -> Result<(), String> {
        Err("service mode is not supported on this platform".into())
    }
    pub async fn stop_service() -> Result<(), String> {
        Ok(())
    }
    pub async fn restart_service() -> Result<(), String> {
        Err("service mode is not supported on this platform".into())
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
pub use unsupported::{
    install, ping, restart_service, start_core, start_service, status, stop_core, stop_service,
    uninstall,
};
