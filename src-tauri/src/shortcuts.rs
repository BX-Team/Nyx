use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub fn register_shortcut(
    app: &AppHandle,
    old_shortcut: Option<&str>,
    new_shortcut: Option<&str>,
    action: &str,
) -> anyhow::Result<()> {
    let manager = app.global_shortcut();

    if let Some(old) = old_shortcut.filter(|s| !s.is_empty()) {
        if let Ok(sc) = old.parse::<Shortcut>() {
            let _ = manager.unregister(sc);
        }
    }

    if let Some(new) = new_shortcut.filter(|s| !s.is_empty()) {
        let sc = new
            .parse::<Shortcut>()
            .map_err(|e| anyhow::anyhow!("invalid shortcut '{}': {}", new, e))?;

        let app_clone = app.clone();
        let action_owned = action.to_string();

        manager.on_shortcut(sc, move |_app, _sc, _event| {
            handle_shortcut_action(&app_clone, &action_owned);
        })?;
    }

    Ok(())
}

fn handle_shortcut_action(app: &AppHandle, action: &str) {
    match action {
        "showWindowShortcut" => crate::windows::main::toggle_main_window(app),

        "triggerSysProxyShortcut" => {
            tauri::async_runtime::spawn({
                let app = app.clone();
                async move {
                    let config_path = crate::utils::dirs::app_config_path();
                    let currently_enabled = std::fs::read_to_string(&config_path)
                        .ok()
                        .and_then(|s| serde_yaml::from_str::<serde_json::Value>(&s).ok())
                        .and_then(|v| v["sysProxy"]["enable"].as_bool())
                        .unwrap_or(false);
                    let new_state = !currently_enabled;
                    let _ = crate::commands::sysproxy::trigger_sys_proxy(new_state, None).await;
                    if let Ok(text) = std::fs::read_to_string(&config_path) {
                        if let Ok(mut val) = serde_yaml::from_str::<serde_json::Value>(&text) {
                            if let Some(sp) = val.get_mut("sysProxy") {
                                sp["enable"] = serde_json::Value::Bool(new_state);
                            }
                            if let Ok(yaml) = serde_yaml::to_string(&val) {
                                let _ = std::fs::write(&config_path, yaml);
                            }
                        }
                    }
                    use tauri::Emitter;
                    let _ = app.emit("app-config-updated", ());
                }
            });
        }

        "triggerTunShortcut" => {
            use tauri::Emitter;
            let _ = app.emit("shortcut-trigger-tun", ());
        }

        "ruleModeShortcut" => {
            tauri::async_runtime::spawn({
                let app = app.clone();
                async move {
                    let _ = crate::core::api::patch_config(serde_json::json!({"mode": "rule"})).await;
                    use tauri::Emitter;
                    let _ = app.emit("app-config-updated", ());
                }
            });
        }

        "globalModeShortcut" => {
            tauri::async_runtime::spawn({
                let app = app.clone();
                async move {
                    let _ = crate::core::api::patch_config(serde_json::json!({"mode": "global"})).await;
                    use tauri::Emitter;
                    let _ = app.emit("app-config-updated", ());
                }
            });
        }

        "directModeShortcut" => {
            tauri::async_runtime::spawn({
                let app = app.clone();
                async move {
                    let _ = crate::core::api::patch_config(serde_json::json!({"mode": "direct"})).await;
                    use tauri::Emitter;
                    let _ = app.emit("app-config-updated", ());
                }
            });
        }

        "restartAppShortcut" => app.restart(),

        "quitWithoutCoreShortcut" => app.exit(0),

        _ => log::warn!("unhandled shortcut action: {}", action),
    }
}
