use nyx_sysproxy::Sysproxy;

/// LAN/loopback ranges excluded from the system proxy.
#[cfg(target_os = "windows")]
const BYPASS: &str = "<local>;localhost;127.*;10.*;172.16.*;172.17.*;172.18.*;172.19.*;172.20.*;172.21.*;172.22.*;172.23.*;172.24.*;172.25.*;172.26.*;172.27.*;172.28.*;172.29.*;172.30.*;172.31.*;192.168.*";
#[cfg(not(target_os = "windows"))]
const BYPASS: &str = "localhost,127.0.0.0/8,::1,10.0.0.0/8,172.16.0.0/12,192.168.0.0/16";

async fn mixed_port() -> u16 {
    crate::backend::api::get_config()
        .await
        .ok()
        .and_then(|c| c["mixed-port"].as_u64())
        .map(|p| p as u16)
        .unwrap_or(7890)
}

pub async fn apply(enable: bool, affect_vpn: bool) {
    let port = if enable { mixed_port().await } else { 0 };
    set_proxy(enable, port, affect_vpn);
}

/// Removes the OS system proxy. Sync — safe to call on the quit path.
pub fn clear() {
    // Clear the same scope we set, or dial-up/VPN entries keep a dead proxy.
    let affect_vpn = crate::backend::config::app_config_bool("affectVPNConnections");
    set_proxy(false, 0, affect_vpn);
}

fn set_proxy(enable: bool, port: u16, affect_vpn: bool) {
    let proxy = Sysproxy {
        enable,
        host: "127.0.0.1".to_string(),
        port,
        bypass: BYPASS.to_string(),
    };
    if let Err(e) = proxy.set_system_proxy_with(affect_vpn) {
        log::warn!("[sysproxy] set(enable={enable}) failed: {e}");
    }

    #[cfg(target_os = "linux")]
    set_env_proxy(enable, port);
}

/// Whether this desktop honours the proxy we write (GNOME family, KDE).
/// Elsewhere only apps reading the env vars follow it, so the toggle is partial.
#[cfg(target_os = "linux")]
pub fn session_honors_proxy() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|d| {
            let d = d.to_ascii_lowercase();
            [
                "gnome", "unity", "cinnamon", "mate", "budgie", "pop", "kde", "plasma",
            ]
            .iter()
            .any(|k| d.contains(k))
        })
        .unwrap_or(false)
}

/// Pushes the proxy env into the systemd user manager and D-Bus activation
/// environment. Does not reach already-running apps.
#[cfg(target_os = "linux")]
fn set_env_proxy(enable: bool, port: u16) {
    const NAMES: [&str; 6] = [
        "http_proxy",
        "https_proxy",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "no_proxy",
        "NO_PROXY",
    ];
    if enable {
        let url = format!("http://127.0.0.1:{port}");
        let no = "localhost,127.0.0.1,::1";
        let assignments = [
            format!("http_proxy={url}"),
            format!("https_proxy={url}"),
            format!("HTTP_PROXY={url}"),
            format!("HTTPS_PROXY={url}"),
            format!("no_proxy={no}"),
            format!("NO_PROXY={no}"),
        ];
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "set-environment"])
            .args(&assignments)
            .status();
        let _ = std::process::Command::new("dbus-update-activation-environment")
            .arg("--systemd")
            .args(&assignments)
            .status();
    } else {
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "unset-environment"])
            .args(NAMES)
            .status();
    }
}
