use serde_json::Value;

use crate::backend::{dirs, manager, mihomo, service};

fn read_app_config_sync() -> Value {
    let path = dirs::app_config_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_yaml::from_str::<Value>(&s).ok())
        .unwrap_or_default()
}

/// One-time data-dir cleanup: rename legacy `config.yaml` → `app-config.yaml`
/// and drop the stale `window-state.json`. Safe on every launch.
pub fn migrate_data_dir() {
    let new = dirs::app_config_path();
    let old = dirs::legacy_app_config_path();
    if !new.exists() && old.exists() {
        match std::fs::rename(&old, &new) {
            Ok(_) => log::info!("[migrate] config.yaml -> app-config.yaml"),
            Err(e) => log::warn!("[migrate] could not rename config.yaml: {e}"),
        }
    }
    let stale = dirs::legacy_window_state_path();
    if stale.exists() {
        let _ = std::fs::remove_file(&stale);
        log::info!("[migrate] removed stale window-state.json");
    }
}

/// Writes the default `app-config.yaml` if none exists yet (first run).
pub fn ensure_default_app_config() {
    let config_path = dirs::app_config_path();
    if config_path.exists() {
        return;
    }
    if let Some(parent) = config_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let defaults = serde_yaml::to_string(&serde_json::json!({
        "silentStart": false,
        "autoCheckUpdate": true,
        "proxyDisplayOrder": "default",
        "proxyDisplayLayout": "double",
        "groupDisplayLayout": "double",
        "proxyCols": "auto",
        "autoCloseConnection": true,
        "useWindowFrame": false,
        "appTheme": "system",
        "maxLogDays": 7,
        "delayTestConcurrency": 50,
        "sysProxy": { "enable": false, "mode": "manual" },
        "hosts": [],
        "core": "mihomo",
        "corePermissionMode": "service"
    }))
    .unwrap_or_default();
    let _ = std::fs::write(&config_path, defaults);
    log::info!("created default app config");
}

/// Brings the core up and confirms it is reachable. On `Ok`, the API client is
/// initialized, `controller_url()` is populated, and selections are restored.
pub async fn start_core_flow() -> Result<(), String> {
    ensure_default_app_config();

    let app_cfg = read_app_config_sync();
    let use_service_mode =
        cfg!(windows) && app_cfg["corePermissionMode"].as_str().unwrap_or("service") == "service";

    if use_service_mode {
        match service::service_status().await {
            Ok(status) if status == "running" || status == "stopped" => {
                service::start_service().await?;
            }
            Ok(status) if status == "not-installed" => {
                return Err(
                    "Service mode is enabled, but the Nyx service is not installed".to_string(),
                );
            }
            Ok(status) => {
                return Err(format!("Unexpected service status: {status}"));
            }
            Err(e) => return Err(e),
        }
    } else {
        let selected_core = app_cfg["core"].as_str().unwrap_or("mihomo");
        manager::install_core_for_core_type(selected_core)
            .await
            .map_err(|e| e.to_string())?;
        manager::start_core().await.map_err(|e| e.to_string())?;
    }

    mihomo::restore_proxy_selections().await;
    log::info!(
        "[startup] core flow complete, controller={}",
        manager::controller_url()
    );
    Ok(())
}
