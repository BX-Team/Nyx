use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use gpui::{App, AsyncApp};
use gpui_component::notification::Notification;

use crate::app::{actions, bootstrap, runtime};
use crate::backend;

/// Registers the `nyx://` URI scheme so the OS launches this exe with the URL. Idempotent.
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

/// Linux: install a `.desktop` entry claiming `x-scheme-handler/nyx` and set it
/// as the default handler. Idempotent.
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
    set_default_handler();
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&apps_dir)
        .status();
}

/// Sets `nyx-url.desktop` as the `x-scheme-handler/nyx` handler by editing
/// `~/.config/mimeapps.list` ourselves. The system `xdg-mime` helper writes
/// its temp file next to the first `mimeapps.list` it finds, which on NixOS is
/// a read-only `/nix/store` path, so it fails there.
#[cfg(target_os = "linux")]
fn set_default_handler() {
    let Some(path) = dirs::config_dir().map(|d| d.join("mimeapps.list")) else {
        return;
    };
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    let updated = upsert_default_application(&current, "x-scheme-handler/nyx", "nyx-url.desktop");
    if let Err(e) = std::fs::write(&path, updated) {
        if e.kind() == std::io::ErrorKind::ReadOnlyFilesystem
            || e.kind() == std::io::ErrorKind::PermissionDenied
        {
            log::info!(
                "[deep-link] {} is read-only (NixOS/home-manager); declare the handler with \
                 xdg.mimeApps.defaultApplications.\"x-scheme-handler/nyx\" = \"nyx-url.desktop\";",
                path.display()
            );
        } else {
            log::warn!("[deep-link] write mimeapps.list failed: {e}");
        }
    }
}

#[cfg(target_os = "linux")]
fn upsert_default_application(contents: &str, key: &str, value: &str) -> String {
    const HEADER: &str = "[Default Applications]";
    let entry = format!("{key}={value}");
    let key_prefix = format!("{key}=");
    let mut lines: Vec<String> = contents.lines().map(str::to_string).collect();

    if let Some(start) = lines.iter().position(|l| l.trim() == HEADER) {
        let end = lines
            .iter()
            .enumerate()
            .skip(start + 1)
            .find(|(_, l)| l.trim_start().starts_with('['))
            .map_or(lines.len(), |(i, _)| i);
        match lines[start + 1..end]
            .iter()
            .position(|l| l.trim_start().starts_with(&key_prefix))
        {
            Some(pos) => lines[start + 1 + pos] = entry,
            None => lines.insert(end, entry),
        }
    } else {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push(HEADER.to_string());
        lines.push(entry);
    }

    let mut out = lines.join("\n");
    out.push('\n');
    out
}

#[cfg(not(any(windows, target_os = "linux")))]
pub fn register_scheme() {}

/// Starts the deep-link drain loop on the gpui main thread, consuming URLs from `rx`.
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

/// Adds a remote profile from `nyx://install-config?url=…` and activates it.
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
