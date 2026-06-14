#[cfg(target_os = "windows")]
pub fn is_elevated() -> bool {
    use std::mem;
    use std::ptr;
    unsafe {
        let mut token: *mut std::ffi::c_void = ptr::null_mut();
        #[link(name = "advapi32")]
        extern "system" {
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
        extern "system" {
            fn GetCurrentProcess() -> *mut std::ffi::c_void;
            fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        }

        const TOKEN_QUERY: u32 = 0x0008;
        const TOKEN_ELEVATION: u32 = 20;

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

#[cfg(not(target_os = "windows"))]
pub fn is_elevated() -> bool {
    unsafe { libc::geteuid() == 0 }
}

/// Net capabilities the mihomo core needs to create a TUN device.
#[cfg(target_os = "linux")]
const TUN_CAPS: [caps::Capability; 3] = [
    caps::Capability::CAP_NET_ADMIN,
    caps::Capability::CAP_NET_BIND_SERVICE,
    caps::Capability::CAP_NET_RAW,
];

/// Raises the TUN net caps into the ambient set so the spawned core inherits
/// them. No-op unless this process already holds them (via `setcap`).
#[cfg(target_os = "linux")]
pub fn raise_net_ambient_caps() {
    use caps::CapSet;
    for cap in TUN_CAPS {
        if caps::raise(None, CapSet::Inheritable, cap).is_err() {
            continue;
        }
        let _ = caps::raise(None, CapSet::Ambient, cap);
    }
}

/// Whether this process already holds `CAP_NET_ADMIN` (so the core can TUN).
#[cfg(target_os = "linux")]
pub fn has_net_admin() -> bool {
    caps::has_cap(
        None,
        caps::CapSet::Permitted,
        caps::Capability::CAP_NET_ADMIN,
    )
    .unwrap_or(false)
}

/// Grants the TUN net caps to the Nyx executable via `pkexec setcap`. Takes
/// effect on the next launch (a running process can't gain file caps live).
#[cfg(target_os = "linux")]
pub fn grant_tun_caps() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let status = std::process::Command::new("pkexec")
        .arg("setcap")
        .arg("cap_net_admin,cap_net_bind_service,cap_net_raw=+ep")
        .arg(&exe)
        .status()
        .map_err(|e| format!("failed to run pkexec/setcap: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("granting TUN privileges was cancelled or failed".to_string())
    }
}
