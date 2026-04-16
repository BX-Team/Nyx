#[tauri::command]
pub async fn trigger_sys_proxy(
    enable: bool,
    affect_vpn_connections: Option<bool>,
) -> Result<(), String> {
    let port: u16 = match crate::core::api::get_config().await {
        Ok(cfg) => cfg["mixed-port"].as_u64().unwrap_or(7890) as u16,
        Err(_) => 7890,
    };
    let proxy_addr = format!("127.0.0.1:{port}");

    #[cfg(target_os = "windows")]
    set_windows_proxy(enable, &proxy_addr, affect_vpn_connections.unwrap_or(false))
        .map_err(|e| e.to_string())?;

    #[cfg(target_os = "linux")]
    set_linux_proxy(enable, &proxy_addr).map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn set_windows_proxy(
    enable: bool,
    proxy_addr: &str,
    affect_vpn_connections: bool,
) -> anyhow::Result<()> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
    let (key, _) = hkcu.create_subkey(path)?;
    
    // Global Proxy settings
    key.set_value("ProxyEnable", &(enable as u32))?;
    if enable {
        key.set_value("ProxyServer", &proxy_addr.to_string())?;
        key.set_value(
            "ProxyOverride",
            &"<local>;localhost;127.*;10.*;172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;192.168.*".to_string(),
        )?;
    }

    if affect_vpn_connections {
        // Apply proxy to all dial-up and VPN connections
        let connections_path = format!("{}\\{}", path, "Connections");
        if let Ok((connections_key, _)) = hkcu.create_subkey(&connections_path) {
            for value_name_res in connections_key.enum_values() {
                if let Ok((name, val)) = value_name_res {
                    if val.vtype != winreg::enums::REG_BINARY { continue; }
                    let mut bytes: Vec<u8> = val.bytes;
                    if bytes.len() < 12 { continue; }
                    
                    // Increment counter
                    let counter = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0,0,0,0]));
                    bytes[4..8].copy_from_slice(&(counter + 1).to_le_bytes());

                    if enable {
                        bytes[8] = 0x03; // PROXY_TYPE_DIRECT | PROXY_TYPE_PROXY
                        let mut new_bytes = bytes[..12].to_vec();
                        
                        let proxy_addr_bytes = proxy_addr.as_bytes();
                        new_bytes.extend_from_slice(&(proxy_addr_bytes.len() as u32).to_le_bytes());
                        new_bytes.extend_from_slice(proxy_addr_bytes);

                        let bypass = "<local>;localhost;127.*;10.*;172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;192.168.*";
                        let bypass_bytes = bypass.as_bytes();
                        new_bytes.extend_from_slice(&(bypass_bytes.len() as u32).to_le_bytes());
                        new_bytes.extend_from_slice(bypass_bytes);
                        
                        // Remaining 36 zeros
                        new_bytes.extend_from_slice(&[0u8; 36]);
                        let _ = connections_key.set_raw_value(&name, &winreg::RegValue { vtype: winreg::enums::REG_BINARY, bytes: new_bytes });
                    } else {
                        bytes[8] = 0x09; // PROXY_TYPE_DIRECT | PROXY_TYPE_AUTO_PROXY_URL
                        // Simplified clear proxy settings
                        let mut new_bytes = bytes[..12].to_vec();
                        new_bytes.extend_from_slice(&[0u8; 8]); // No Proxy Addr, No Bypass
                        new_bytes.extend_from_slice(&[0u8; 36]);
                        let _ = connections_key.set_raw_value(&name, &winreg::RegValue { vtype: winreg::enums::REG_BINARY, bytes: new_bytes });
                    }
                }
            }
        }
    }

    // Notify OS of settings change
    unsafe {
        #[link(name = "wininet")]
        extern "system" {
            fn InternetSetOptionW(hInternet: *mut std::ffi::c_void, dwOption: u32, lpBuffer: *mut std::ffi::c_void, dwBufferLength: u32) -> i32;
        }
        InternetSetOptionW(std::ptr::null_mut(), 39, std::ptr::null_mut(), 0); // INTERNET_OPTION_SETTINGS_CHANGED
        InternetSetOptionW(std::ptr::null_mut(), 37, std::ptr::null_mut(), 0); // INTERNET_OPTION_REFRESH
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn set_linux_proxy(enable: bool, proxy_addr: &str) -> anyhow::Result<()> {
    if enable {
        let (host, port) = proxy_addr.split_once(':').unwrap_or(("127.0.0.1", "7890"));
        let _ = std::process::Command::new("gsettings")
            .args(["set", "org.gnome.system.proxy", "mode", "manual"])
            .status();
        for schema in ["http", "https"] {
            let _ = std::process::Command::new("gsettings")
                .args(["set", &format!("org.gnome.system.proxy.{schema}"), "host", host])
                .status();
            let _ = std::process::Command::new("gsettings")
                .args(["set", &format!("org.gnome.system.proxy.{schema}"), "port", port])
                .status();
        }
    } else {
        let _ = std::process::Command::new("gsettings")
            .args(["set", "org.gnome.system.proxy", "mode", "none"])
            .status();
    }
    Ok(())
}
