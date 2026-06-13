/// LAN/loopback ranges excluded from the system proxy (Windows only).
#[cfg(target_os = "windows")]
const BYPASS: &str = "<local>;localhost;127.*;10.*;172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;192.168.*";

/// `127.0.0.1:<mixed-port>` from the live core config (falls back to 7890).
async fn proxy_addr() -> String {
    let port = crate::backend::api::get_config()
        .await
        .ok()
        .and_then(|c| c["mixed-port"].as_u64())
        .unwrap_or(7890);
    format!("127.0.0.1:{port}")
}

/// Applies (or removes) the OS system proxy pointing at the mixed port.
pub async fn apply(enable: bool, affect_vpn: bool) {
    let addr = if enable {
        proxy_addr().await
    } else {
        String::new()
    };
    set_proxy(enable, &addr, affect_vpn);
}

/// Removes the OS system proxy. Sync — safe to call on the quit path.
pub fn clear() {
    set_proxy(false, "", false);
}

#[cfg(target_os = "windows")]
fn set_proxy(enable: bool, proxy_addr: &str, affect_vpn: bool) {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
    let Ok((key, _)) = hkcu.create_subkey(path) else {
        log::warn!("[sysproxy] could not open Internet Settings key");
        return;
    };

    let _ = key.set_value("ProxyEnable", &(enable as u32));
    if enable {
        let _ = key.set_value("ProxyServer", &proxy_addr.to_string());
        let _ = key.set_value("ProxyOverride", &BYPASS.to_string());
    }

    if affect_vpn {
        set_windows_connections(&hkcu, path, enable, proxy_addr);
    }

    unsafe {
        #[link(name = "wininet")]
        extern "system" {
            fn InternetSetOptionW(
                h: *mut std::ffi::c_void,
                opt: u32,
                buf: *mut std::ffi::c_void,
                len: u32,
            ) -> i32;
        }
        // INTERNET_OPTION_SETTINGS_CHANGED, then INTERNET_OPTION_REFRESH.
        InternetSetOptionW(std::ptr::null_mut(), 39, std::ptr::null_mut(), 0);
        InternetSetOptionW(std::ptr::null_mut(), 37, std::ptr::null_mut(), 0);
    }
}

#[cfg(target_os = "windows")]
fn set_windows_connections(hkcu: &winreg::RegKey, path: &str, enable: bool, proxy_addr: &str) {
    let connections_path = format!("{path}\\Connections");
    let Ok((conns, _)) = hkcu.create_subkey(&connections_path) else {
        return;
    };
    for (name, val) in conns.enum_values().flatten() {
        if val.vtype != winreg::enums::REG_BINARY || val.bytes.len() < 12 {
            continue;
        }
        let mut head = val.bytes[..12].to_vec();
        let counter = u32::from_le_bytes(head[4..8].try_into().unwrap_or([0; 4]));
        head[4..8].copy_from_slice(&(counter + 1).to_le_bytes());

        let mut bytes = head;
        if enable {
            bytes[8] = 0x03; // DIRECT | PROXY
            bytes.extend_from_slice(&(proxy_addr.len() as u32).to_le_bytes());
            bytes.extend_from_slice(proxy_addr.as_bytes());
            bytes.extend_from_slice(&(BYPASS.len() as u32).to_le_bytes());
            bytes.extend_from_slice(BYPASS.as_bytes());
        } else {
            bytes[8] = 0x01; // DIRECT only
            bytes.extend_from_slice(&[0u8; 8]);
        }
        bytes.extend_from_slice(&[0u8; 36]);
        let _ = conns.set_raw_value(
            &name,
            &winreg::RegValue {
                vtype: winreg::enums::REG_BINARY,
                bytes,
            },
        );
    }
}

#[cfg(target_os = "linux")]
fn set_proxy(enable: bool, proxy_addr: &str, _affect_vpn: bool) {
    let run = |args: &[&str]| {
        let _ = std::process::Command::new("gsettings").args(args).status();
    };
    if enable {
        let (host, port) = proxy_addr.split_once(':').unwrap_or(("127.0.0.1", "7890"));
        run(&["set", "org.gnome.system.proxy", "mode", "manual"]);
        for schema in ["http", "https"] {
            run(&[
                "set",
                &format!("org.gnome.system.proxy.{schema}"),
                "host",
                host,
            ]);
            run(&[
                "set",
                &format!("org.gnome.system.proxy.{schema}"),
                "port",
                port,
            ]);
        }
    } else {
        run(&["set", "org.gnome.system.proxy", "mode", "none"]);
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn set_proxy(_enable: bool, _proxy_addr: &str, _affect_vpn: bool) {}
