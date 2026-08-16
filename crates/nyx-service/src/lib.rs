mod control;
mod host;
mod logging;
mod protocol;

pub use control::{
    Status, install, is_managed, ping, restart_service, start_core, start_service, status,
    stop_core, stop_service, uninstall,
};
pub use protocol::{CoreSpec, PROTOCOL_VERSION};

pub const SERVICE_NAME: &str = "Nyx Service";
pub const SERVICE_DISPLAY_NAME: &str = "Nyx Mihomo Service";

pub(crate) const ARG_HOST: &str = "--nyx-service";
pub(crate) const ARG_INSTALL: &str = "--nyx-service-install";
pub(crate) const ARG_UNINSTALL: &str = "--nyx-service-uninstall";
pub(crate) const ARG_CONTROL: &str = "--nyx-service-control";
pub(crate) const ARG_OWNER: &str = "--nyx-service-owner";

/// "prompt dismissed" and "authorisation refused".
pub(crate) const HELPER_FAILURE: i32 = 9;

/// Handles the argv modes that must run before any GUI work: the service host
/// and the two elevated install/uninstall entry points.
pub fn maybe_run_service_mode() -> Option<i32> {
    let args: Vec<String> = std::env::args().collect();
    let has = |flag: &str| args.iter().any(|a| a == flag);

    if has(ARG_HOST) {
        return Some(run_host());
    }
    if has(ARG_INSTALL) {
        return Some(report(install_here()));
    }
    if has(ARG_UNINSTALL) {
        return Some(report(uninstall_here()));
    }
    if has(ARG_CONTROL) {
        return Some(report(control_here(
            &flag_value(ARG_CONTROL).unwrap_or_default(),
        )));
    }
    None
}

fn report(result: Result<(), String>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(e) => {
            logging::log(&format!("privileged helper failed: {e}"));
            eprintln!("{e}");
            HELPER_FAILURE
        }
    }
}

#[cfg(windows)]
fn run_host() -> i32 {
    host::windows::run_dispatcher()
}

#[cfg(target_os = "linux")]
fn run_host() -> i32 {
    let owner = match owner_arg() {
        Some(value) => resolve_uid(&value).unwrap_or_else(|| {
            logging::log(&format!(
                "unknown service owner {value:?} — only root may drive the service"
            ));
            0
        }),
        None => 0,
    };
    host::linux::run_host(owner)
}

/// The owner arrives as a uid from Nyx's own installer, but as a user name from
/// a declarative unit, where the uid is not known at build time.
#[cfg(target_os = "linux")]
fn resolve_uid(value: &str) -> Option<u32> {
    if let Ok(uid) = value.parse::<u32>() {
        return Some(uid);
    }
    let name = std::ffi::CString::new(value).ok()?;
    // getpwnam's static buffer is safe here: this runs once, before any threads.
    let pw = unsafe { libc::getpwnam(name.as_ptr()) };
    (!pw.is_null()).then(|| unsafe { (*pw).pw_uid })
}

#[cfg(not(any(windows, target_os = "linux")))]
fn run_host() -> i32 {
    1
}

#[cfg(windows)]
fn install_here() -> Result<(), String> {
    control::windows::install_here()
}

#[cfg(target_os = "linux")]
fn install_here() -> Result<(), String> {
    let uid = owner_arg()
        .and_then(|s| resolve_uid(&s))
        .or_else(|| {
            std::env::var("PKEXEC_UID")
                .ok()
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or_else(|| unsafe { libc::getuid() });
    control::linux::install_here(uid)
}

#[cfg(not(any(windows, target_os = "linux")))]
fn install_here() -> Result<(), String> {
    Err("service mode is not supported on this platform".into())
}

#[cfg(windows)]
fn uninstall_here() -> Result<(), String> {
    control::windows::uninstall_here()
}

#[cfg(target_os = "linux")]
fn uninstall_here() -> Result<(), String> {
    control::linux::uninstall_here()
}

#[cfg(not(any(windows, target_os = "linux")))]
fn uninstall_here() -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
fn control_here(action: &str) -> Result<(), String> {
    control::windows::control_here(action)
}

#[cfg(target_os = "linux")]
fn control_here(action: &str) -> Result<(), String> {
    control::linux::control_here(action)
}

#[cfg(not(any(windows, target_os = "linux")))]
fn control_here(_action: &str) -> Result<(), String> {
    Err("service mode is not supported on this platform".into())
}

fn flag_value(flag: &str) -> Option<String> {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == flag {
            return args.next().filter(|s| !s.is_empty());
        }
    }
    None
}

#[cfg(any(windows, target_os = "linux"))]
pub(crate) fn owner_arg() -> Option<String> {
    flag_value(ARG_OWNER)
}

#[cfg(windows)]
pub fn is_elevated() -> bool {
    use std::mem;
    use std::ptr;
    unsafe {
        #[link(name = "advapi32")]
        unsafe extern "system" {
            fn OpenProcessToken(
                process: *mut std::ffi::c_void,
                desired_access: u32,
                token_handle: *mut *mut std::ffi::c_void,
            ) -> i32;
            fn GetTokenInformation(
                token_handle: *mut std::ffi::c_void,
                token_information_class: u32,
                token_information: *mut std::ffi::c_void,
                token_information_length: u32,
                return_length: *mut u32,
            ) -> i32;
        }
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetCurrentProcess() -> *mut std::ffi::c_void;
            fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        }

        const TOKEN_QUERY: u32 = 0x0008;
        const TOKEN_ELEVATION: u32 = 20;

        let mut token: *mut std::ffi::c_void = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }

        #[repr(C)]
        struct TokenElevation {
            token_is_elevated: u32,
        }
        let mut elevation: TokenElevation = mem::zeroed();
        let mut size: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TOKEN_ELEVATION,
            &mut elevation as *mut _ as *mut std::ffi::c_void,
            mem::size_of::<TokenElevation>() as u32,
            &mut size,
        );
        CloseHandle(token);
        ok != 0 && elevation.token_is_elevated != 0
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    unsafe { libc::geteuid() == 0 }
}
