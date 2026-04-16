use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::Value;
use tauri::AppHandle;

static PENDING_UPDATE: Lazy<Mutex<Option<tauri_plugin_updater::Update>>> =
    Lazy::new(|| Mutex::new(None));

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Value, String> {
    use tauri_plugin_updater::UpdaterExt;

    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;

    if let Some(update) = update {
        let info = serde_json::json!({
            "version": update.version,
            "currentVersion": update.current_version,
            "changelog": update.body.clone().unwrap_or_default(),
        });
        *PENDING_UPDATE.lock() = Some(update);
        Ok(info)
    } else {
        Ok(Value::Null)
    }
}

#[tauri::command]
pub async fn download_and_install_update(_version: String) -> Result<(), String> {
    let update = PENDING_UPDATE
        .lock()
        .take()
        .ok_or("no pending update — call check_update first")?;

    #[cfg(windows)]
    {
        let _ = crate::commands::service::stop_service_for_update().await;
    }

    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel_update() -> Result<(), String> {
    *PENDING_UPDATE.lock() = None;
    Ok(())
}
