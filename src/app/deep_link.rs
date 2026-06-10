use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use gpui::{App, AsyncApp};
use gpui_component::notification::Notification;

use crate::app::{actions, bootstrap, runtime};
use crate::backend;

/// Registers the `nyx://` URI scheme so the OS launches this exe with the URL as
/// its argument. Idempotent; call once on primary-instance startup.
#[cfg(windows)]
pub fn register_scheme() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let exe = exe.to_string_lossy().into_owned();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let res = (|| -> std::io::Result<()> {
        let (scheme, _) = hkcu.create_subkey(r"Software\Classes\nyx")?;
        scheme.set_value("", &"URL:Nyx Protocol")?;
        scheme.set_value("URL Protocol", &"")?;
        let (cmd, _) = hkcu.create_subkey(r"Software\Classes\nyx\shell\open\command")?;
        cmd.set_value("", &format!("\"{exe}\" \"%1\""))?;
        Ok(())
    })();
    if let Err(e) = res {
        log::warn!("[deep-link] scheme registration failed: {e}");
    }
}

/// Linux: install a `.desktop` entry that claims the `x-scheme-handler/nyx`
/// MIME type, then make it the default handler so the desktop launches Nyx for
/// `nyx://` links. Idempotent.
#[cfg(target_os = "linux")]
pub fn register_scheme() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(apps_dir) = dirs::data_dir().map(|d| d.join("applications")) else {
        return;
    };
    if let Err(e) = std::fs::create_dir_all(&apps_dir) {
        log::warn!("[deep-link] mkdir applications failed: {e}");
        return;
    }
    let desktop = apps_dir.join("nyx-url.desktop");
    let contents = format!(
        "[Desktop Entry]\n\
         Name=Nyx\n\
         Exec={} %u\n\
         Type=Application\n\
         Terminal=false\n\
         NoDisplay=true\n\
         MimeType=x-scheme-handler/nyx;\n",
        exe.to_string_lossy()
    );
    if let Err(e) = std::fs::write(&desktop, contents) {
        log::warn!("[deep-link] write .desktop failed: {e}");
        return;
    }
    let _ = std::process::Command::new("xdg-mime")
        .args(["default", "nyx-url.desktop", "x-scheme-handler/nyx"])
        .status();
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&apps_dir)
        .status();
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn register_scheme() {}

/// Starts the deep-link drain loop on the gpui main thread, consuming URLs that
/// `single_instance::serve` (and our own launch arg) push onto `rx`.
pub fn start(rx: Receiver<String>, cx: &mut App) {
    cx.spawn(async move |cx: &mut AsyncApp| loop {
        cx.background_executor()
            .timer(Duration::from_millis(150))
            .await;
        while let Ok(url) = rx.try_recv() {
            cx.update(|cx| handle_url(&url, cx));
        }
    })
    .detach();
}

/// Parses and dispatches a single `nyx://` URL.
fn handle_url(url: &str, cx: &mut App) {
    let Ok(parsed) = url::Url::parse(url) else {
        log::warn!("[deep-link] failed to parse: {url}");
        return;
    };
    // `nyx://install-config?…` puts the command in the host; tolerate a path too.
    let host = parsed.host_str().unwrap_or("").to_string();
    let path_cmd = parsed.path().trim_start_matches('/').to_string();
    let command = if host.is_empty() { path_cmd } else { host };
    let params: HashMap<String, String> = parsed.query_pairs().into_owned().collect();
    log::info!("[deep-link] command='{command}' params={params:?}");

    actions::show_window(cx);
    match command.as_str() {
        "install-config" => install_config(params, cx),
        other => log::warn!("[deep-link] unknown command '{other}'"),
    }
}

/// Adds a remote profile from `nyx://install-config?url=…`, activates it, and
/// toasts the outcome.
fn install_config(params: HashMap<String, String>, cx: &mut App) {
    let Some(config_url) = params.get("url").cloned() else {
        log::warn!("[deep-link] install-config: missing 'url'");
        return;
    };
    let name = params.get("name").cloned().unwrap_or_default();
    cx.spawn(async move |cx: &mut AsyncApp| {
        let item = serde_json::json!({ "type": "remote", "url": config_url, "name": name });
        match runtime::spawn(backend::config::add_profile_item(item)).await {
            Ok(Ok(id)) => {
                let _ = runtime::spawn(backend::config::change_current_profile(id)).await;
                bootstrap::refresh_runtime_data(cx).await;
                cx.update(|cx| actions::notify(Notification::success("Profile installed"), cx));
            }
            Ok(Err(e)) => {
                log::error!("[deep-link] install-config failed: {e}");
                cx.update(|cx| {
                    actions::notify(Notification::error(format!("Install failed: {e}")), cx)
                });
            }
            Err(_) => log::error!("[deep-link] install task cancelled"),
        }
    })
    .detach();
}
