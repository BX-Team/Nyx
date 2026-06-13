pub mod api;
pub mod config;
pub mod dirs;
pub mod elevation;
pub mod manager;
pub mod mihomo;
pub mod proxy_convert;
pub mod service;
pub mod service_host;
pub mod startup;
pub mod streaming;
pub mod sysproxy;
pub mod updater;

#[cfg(windows)]
pub use service_host::maybe_run_as_service_from_args;

#[cfg(not(windows))]
pub fn maybe_run_as_service_from_args() -> Option<i32> {
    None
}
