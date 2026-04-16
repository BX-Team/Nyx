use tauri::AppHandle;

#[tauri::command]
pub async fn show_tray_icon(app: AppHandle) -> Result<(), String> {
    if app.tray_by_id("main").is_none() {
        crate::tray::setup_tray(&app).map_err(|e| e.to_string())?;
    } else if let Some(tray) = app.tray_by_id("main") {
        tray.set_visible(true).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn close_tray_icon(app: AppHandle) -> Result<(), String> {
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_visible(false).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn refresh_tray(app: &AppHandle) {
    let (tun_enabled, mode) = match crate::core::api::get_config().await {
        Ok(cfg) => (
            cfg["tun"]["enable"].as_bool().unwrap_or(false),
            cfg["mode"].as_str().unwrap_or("rule").to_string(),
        ),
        Err(_) => (false, String::new()),
    };

    let profile_name = crate::commands::config::get_current_profile_item()
        .await
        .ok()
        .and_then(|v| v["name"].as_str().map(|s| s.to_string()))
        .unwrap_or_default();

    crate::tray::update_tray_icon(app, tun_enabled);
    crate::tray::update_tray_tooltip(app, &profile_name, &mode, tun_enabled);
}

#[tauri::command]
pub async fn update_tray_icon(app: AppHandle) -> Result<(), String> {
    refresh_tray(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn set_dock_visible(_app: AppHandle, _visible: bool) {
}

#[tauri::command]
pub async fn copy_env(app: AppHandle, env_type: String) -> Result<(), String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;

    let port: u16 = match crate::core::api::get_config().await {
        Ok(cfg) => cfg["mixed-port"].as_u64().unwrap_or(7890) as u16,
        Err(_) => 7890,
    };

    let host = "127.0.0.1";
    let http_addr = format!("http://{host}:{port}");
    let socks_addr = format!("socks5://{host}:{port}");

    let text = match env_type.as_str() {
        "bash" => format!(
            "export http_proxy=\"{http_addr}\"\nexport https_proxy=\"{http_addr}\"\nexport all_proxy=\"{socks_addr}\""
        ),
        "cmd" => format!(
            "set http_proxy={http_addr}\r\nset https_proxy={http_addr}"
        ),
        "powershell" => format!(
            "$Env:http_proxy=\"{http_addr}\"\n$Env:https_proxy=\"{http_addr}\""
        ),
        _ => http_addr,
    };

    app.clipboard().write_text(text).map_err(|e| e.to_string())
}
