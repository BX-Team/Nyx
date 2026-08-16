#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Sysproxy {
    pub host: String,
    pub bypass: String,
    pub port: u16,
    pub enable: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Autoproxy {
    pub url: String,
    pub enable: bool,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to parse string `{0}`")]
    ParseStr(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("failed to set proxy for this environment")]
    NotSupport,

    #[cfg(target_os = "windows")]
    #[error("Windows system call failed: {0}")]
    SystemCall(#[from] ::windows::core::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Sysproxy {
    pub const fn is_support() -> bool {
        cfg!(any(target_os = "linux", target_os = "windows"))
    }
}

impl Autoproxy {
    pub const fn is_support() -> bool {
        cfg!(any(target_os = "linux", target_os = "windows"))
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
impl Sysproxy {
    pub fn get_system_proxy() -> Result<Sysproxy> {
        Err(Error::NotSupport)
    }

    pub fn set_system_proxy(&self) -> Result<()> {
        Err(Error::NotSupport)
    }

    pub fn set_system_proxy_with(&self, _include_ras: bool) -> Result<()> {
        Err(Error::NotSupport)
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
impl Autoproxy {
    pub fn get_auto_proxy() -> Result<Autoproxy> {
        Err(Error::NotSupport)
    }

    pub fn set_auto_proxy(&self) -> Result<()> {
        Err(Error::NotSupport)
    }
}
