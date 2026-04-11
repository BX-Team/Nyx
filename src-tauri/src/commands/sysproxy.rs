#[tauri::command]
pub async fn trigger_sys_proxy(
    enable: bool,
    only_active_device: Option<bool>,
) -> Result<(), String> {
    let _ = only_active_device;

    let port: u16 = match crate::core::api::get_config().await {
        Ok(cfg) => cfg["mixed-port"].as_u64().unwrap_or(7890) as u16,
        Err(_) => 7890,
    };
    let proxy_addr = format!("127.0.0.1:{port}");

    #[cfg(target_os = "windows")]
    set_windows_proxy(enable, &proxy_addr).map_err(|e| e.to_string())?;

    #[cfg(target_os = "linux")]
    set_linux_proxy(enable, &proxy_addr).map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn set_windows_proxy(enable: bool, proxy_addr: &str) -> anyhow::Result<()> {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
    let (key, _) = hkcu.create_subkey(path)?;
    key.set_value("ProxyEnable", &(enable as u32))?;
    if enable {
        key.set_value("ProxyServer", &proxy_addr.to_string())?;
        key.set_value(
            "ProxyOverride",
            &"<local>;localhost;127.*;10.*;172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;192.168.*".to_string(),
        )?;
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
