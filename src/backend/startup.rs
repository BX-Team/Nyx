use serde_json::Value;

use crate::backend::core::CoreError;
use crate::backend::{core, dirs, mihomo};

/// Renames the legacy `config.yaml` and drops the stale window state.
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

/// Brings the core up and confirms it answers; also restores proxy selections.
pub async fn start_core_flow() -> Result<(), CoreError> {
    ensure_default_app_config();
    core::start().await?;
    mihomo::restore_proxy_selections().await;
    log::info!(
        "[startup] core flow complete, controller={}",
        core::controller_url()
    );
    Ok(())
}

pub fn read_app_config_sync() -> Value {
    std::fs::read_to_string(dirs::app_config_path())
        .ok()
        .and_then(|s| serde_yaml::from_str::<Value>(&s).ok())
        .unwrap_or_default()
}
