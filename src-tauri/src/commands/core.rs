use tauri::AppHandle;

#[tauri::command]
pub async fn restart_core(app: AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    log::info!("[cmd::restart_core] called");

    #[cfg(windows)]
    if crate::commands::service::service_status().await == Ok("running".to_string()) {
        log::info!("[cmd::restart_core] redirecting to service restart");
        return crate::commands::service::restart_service(app).await;
    }

    crate::core::streaming::stop_streaming();
    match crate::core::manager::restart_core().await {
        Ok(url) => {
            log::info!("[cmd::restart_core] success, url={url}");
            crate::commands::mihomo::restore_proxy_selections().await;
            let _ = app.emit("core-started", ());
            let _ = app.emit("controled-mihomo-config-updated", ());
            crate::core::streaming::start_streaming(&app);
            Ok(())
        }
        Err(e) => {
            log::error!("[cmd::restart_core] failed: {e}");
            Err(e.to_string())
        }
    }
}

use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};

static NETWORK_DETECTION_RUNNING: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

#[tauri::command]
pub async fn start_network_detection(app: AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    if NETWORK_DETECTION_RUNNING.swap(true, Ordering::SeqCst) {
        return Ok(()); 
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();
        while NETWORK_DETECTION_RUNNING.load(Ordering::SeqCst) {
            let reachable = client
                .get("https://www.gstatic.com/generate_204")
                .send()
                .await
                .map(|r| r.status().is_success() || r.status().as_u16() == 204)
                .unwrap_or(false);
            let _ = handle.emit("network-status", reachable);
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn stop_network_detection(_app: AppHandle) -> Result<(), String> {
    NETWORK_DETECTION_RUNNING.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn manual_grant_core_permition(_cores: Option<Vec<String>>) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(vm) = mihomo_rs::VersionManager::with_home(crate::utils::dirs::data_dir())
            .map_err(|e| anyhow::anyhow!("{e}"))
        {
            if let Ok(binary) = vm.get_binary_path(None).await {
                let _ = std::process::Command::new("chmod")
                    .args(["+x", &binary.to_string_lossy()])
                    .status();
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn check_core_permission() -> Result<bool, String> {
    #[cfg(target_os = "linux")]
    {
        if let Ok(vm) = mihomo_rs::VersionManager::with_home(crate::utils::dirs::data_dir())
            .map_err(|e| anyhow::anyhow!("{e}"))
        {
            if let Ok(binary) = vm.get_binary_path(None).await {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(&binary) {
                    return Ok(meta.permissions().mode() & 0o111 != 0);
                }
            }
        }
        return Ok(false);
    }
    #[cfg(not(target_os = "linux"))]
    Ok(true)
}

#[tauri::command]
pub async fn revoke_core_permission(_cores: Option<Vec<String>>) -> Result<(), String> {
    Ok(())
}
