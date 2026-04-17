use tauri::AppHandle;

const SERVICE_NAME: &str = "Nyx Service";
const SERVICE_DISPLAY_NAME: &str = "Nyx Mihomo Service";

#[cfg(windows)]
fn run_sc(args: &[String]) -> Result<std::process::Output, String> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("sc")
        .args(args)
        .creation_flags(0x08000000) 
        .output()
        .map_err(|e| e.to_string())
}

#[cfg(windows)]
fn output_message(out: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !stderr.is_empty() {
        stderr
    } else {
        stdout
    }
}

#[cfg(windows)]
fn service_query_state() -> Result<Option<String>, String> {
    use windows_service::service::{ServiceAccess, ServiceState};
    use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|e| format!("failed to open service manager: {e}"))?;

    let service = match manager.open_service(SERVICE_NAME, ServiceAccess::QUERY_STATUS) {
        Ok(s) => s,
        Err(windows_service::Error::Winapi(ref e)) if e.raw_os_error() == Some(1060) => {
            return Ok(None);
        }
        Err(e) => return Err(format!("failed to open service: {e}")),
    };

    let status = service
        .query_status()
        .map_err(|e| format!("failed to query service status: {e}"))?;

    Ok(Some(match status.current_state {
        ServiceState::Running | ServiceState::StartPending | ServiceState::ContinuePending => {
            "running".to_string()
        }
        ServiceState::Stopped | ServiceState::StopPending => "stopped".to_string(),
        ServiceState::Paused | ServiceState::PausePending => "stopped".to_string(),
    }))
}

#[cfg(windows)]
async fn ensure_core_binary() -> Result<std::path::PathBuf, String> {
    let mut selected_core = "mihomo".to_string();
    let app_cfg_path = crate::utils::dirs::app_config_path();
    if let Ok(cfg_text) = tokio::fs::read_to_string(&app_cfg_path).await {
        if let Ok(cfg) = serde_yaml::from_str::<serde_yaml::Value>(&cfg_text) {
            let core = cfg
                .get("core")
                .and_then(|v| v.as_str())
                .unwrap_or("mihomo");
            selected_core = core.to_string();
            if core == "system" {
                let path = cfg
                    .get("systemCorePath")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| "system core path is not configured".to_string())?;
                let p = std::path::PathBuf::from(path);
                if !p.exists() {
                    return Err(format!("system core does not exist: {}", p.display()));
                }
                return Ok(p);
            }
        }
    }

    crate::core::manager::install_core_for_core_type(&selected_core)
        .await
        .map_err(|e| e.to_string())?;

    let vm = mihomo_rs::VersionManager::with_home(crate::utils::dirs::data_dir())
        .map_err(|e| e.to_string())?;
    vm.get_binary_path(None).await.map_err(|e| e.to_string())
}

#[cfg(windows)]
async fn ensure_runtime_config() -> Result<(std::path::PathBuf, String), String> {
    let url = crate::core::manager::rebuild_config()
        .await
        .map_err(|e| e.to_string())?;
    let cm = mihomo_rs::ConfigManager::with_home(crate::utils::dirs::data_dir())
        .map_err(|e| e.to_string())?;
    let config = cm.get_current_path().await.map_err(|e| e.to_string())?;
    Ok((config, url))
}

#[cfg(windows)]
fn build_service_binpath() -> Result<String, String> {
    let service_host_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    Ok(format!(
        "\"{}\" --nyx-service",
        service_host_exe.display()
    ))
}

#[cfg(windows)]
fn read_secret_from_config(config: &std::path::Path) -> Option<String> {
    let content = std::fs::read_to_string(config).ok()?;
    let val: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    val.get("secret")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

#[cfg(windows)]
fn read_max_log_days() -> u32 {
    let path = crate::utils::dirs::app_config_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_yaml::from_str::<serde_yaml::Value>(&s).ok())
        .and_then(|v| v.get("maxLogDays").and_then(|x| x.as_u64()))
        .map(|d| d as u32)
        .unwrap_or(7)
}

#[cfg(not(windows))]
fn read_secret_from_config(_config: &std::path::Path) -> Option<String> {
    None
}

#[cfg(windows)]
fn sync_controller(url: &str, config: &std::path::Path) -> Result<(), String> {
    crate::core::manager::set_controller_url(url.to_string());
    crate::core::api::init_client(url, read_secret_from_config(config)).map_err(|e| e.to_string())
}

#[cfg(windows)]
fn ensure_service_installed_args() -> Result<(), String> {
    let bin_path = build_service_binpath()?;
    match service_query_state()? {
        Some(_) => {
            let out = run_sc(&[
                "config".to_string(),
                SERVICE_NAME.to_string(),
                "binPath=".to_string(),
                bin_path,
                "start=".to_string(),
                "auto".to_string(),
                "DisplayName=".to_string(),
                SERVICE_DISPLAY_NAME.to_string(),
            ])?;
            if !out.status.success() {
                return Err(format!("failed to reconfigure service: {}", output_message(&out)));
            }
            Ok(())
        }
        None => {
            let out = run_sc(&[
                "create".to_string(),
                SERVICE_NAME.to_string(),
                "binPath=".to_string(),
                bin_path,
                "start=".to_string(),
                "auto".to_string(),
                "DisplayName=".to_string(),
                SERVICE_DISPLAY_NAME.to_string(),
            ])?;
            if !out.status.success() {
                return Err(format!("failed to create service: {}", output_message(&out)));
            }
            Ok(())
        }
    }
}

#[cfg(windows)]
fn sc_start_service() -> Result<(), String> {
    let out = run_sc(&["start".to_string(), SERVICE_NAME.to_string()])?;
    if out.status.success() {
        return Ok(());
    }
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if text.contains("1056") {
        return Ok(());
    }
    Err(format!("failed to start service: {}", output_message(&out)))
}

#[cfg(windows)]
fn sc_stop_service() -> Result<(), String> {
    let out = run_sc(&["stop".to_string(), SERVICE_NAME.to_string()])?;
    if out.status.success() {
        return Ok(());
    }
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if text.contains("1062") {
        return Ok(());
    }
    Err(format!("failed to stop service: {}", output_message(&out)))
}

#[cfg(windows)]
async fn wait_for_service_state(expected_running: bool) -> Result<(), String> {
    for _ in 0..20 {
        if let Ok(Some(state)) = service_query_state() {
            if expected_running && state == "running" {
                return Ok(());
            }
            if !expected_running && state == "stopped" {
                return Ok(());
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
    Err(format!(
        "service did not reach {} state within timeout",
        if expected_running { "running" } else { "stopped" }
    ))
}

#[cfg(windows)]
async fn send_ipc_request(req: &crate::service_host::IpcRequest) -> Result<(), String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::windows::named_pipe::ClientOptions;
    
    let mut client = ClientOptions::new().open(crate::service_host::IPC_PIPE_NAME).map_err(|e| e.to_string())?;
    
    let req_str = serde_json::to_string(req).map_err(|_e| "failed to serialize IPC request".to_string())?;
    client.write_all(req_str.as_bytes()).await.map_err(|e| e.to_string())?;
    
    let mut buf = vec![0u8; 4096];
    let n = client.read(&mut buf).await.map_err(|e| e.to_string())?;
    let msg = String::from_utf8_lossy(&buf[..n]);
    
    if let Ok(res) = serde_json::from_str::<crate::service_host::IpcResponse>(&msg) {
        match res {
            crate::service_host::IpcResponse::Ok | crate::service_host::IpcResponse::Pong => Ok(()),
            crate::service_host::IpcResponse::Error { message } => Err(message),
        }
    } else {
        Err("invalid IPC response".to_string())
    }
}

#[cfg(windows)]
async fn start_windows_service(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    let binary = ensure_core_binary().await?;
    let (config, url) = ensure_runtime_config().await?;
    let work_dir = config.parent().unwrap_or(std::path::Path::new("")).to_string_lossy().into_owned();

    let state = service_query_state()?;
    if state != Some("running".to_string()) {
        if !crate::commands::window::is_elevated_sync() {
            let bat_cmd = format!(
                "@echo off\r\nchcp 65001 > nul\r\nsc.exe start \"{SERVICE_NAME}\"\r\nif errorlevel 1 exit /b 1\r\n",
                SERVICE_NAME = SERVICE_NAME
            );
            run_elevated_bat(&bat_cmd)?;
        } else {
            sc_start_service()?;
        }
        wait_for_service_state(true).await?;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    let req = crate::service_host::IpcRequest::StartCore {
        binary: binary.to_string_lossy().into_owned(),
        work_dir,
        config: config.to_string_lossy().into_owned(),
        max_log_days: read_max_log_days(),
    };

    send_ipc_request(&req).await?;

    sync_controller(&url, &config)?;
    let _ = app.emit("core-started", ());
    let _ = app.emit("controled-mihomo-config-updated", ());
    crate::core::streaming::start_streaming(app);
    Ok(())
}

async fn is_mihomo_running() -> bool {
    let mut url = crate::core::manager::controller_url();
    if url.is_empty() {
        if let Ok(cm) = mihomo_rs::ConfigManager::with_home(crate::utils::dirs::data_dir()) {
            if let Ok(config) = cm.get_current_path().await {
                if let Ok(content) = tokio::fs::read_to_string(&config).await {
                    if let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
                        if let Some(addr) = val.get("external-controller").and_then(|v| v.as_str()) {
                            if addr.starts_with("http") {
                                url = addr.to_string();
                            } else if addr.starts_with(':') {
                                url = format!("http://127.0.0.1{addr}");
                            } else {
                                url = format!("http://{addr}");
                            }
                            crate::core::manager::set_controller_url(url.clone());
                            let _ = crate::core::api::init_client(&url, read_secret_from_config(&config));
                        }
                    }
                }
            }
        }
    }
    if url.is_empty() {
        return false;
    }
    let version_url = format!("{url}/version");
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default()
        .get(&version_url)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

#[tauri::command]
pub async fn service_status() -> Result<String, String> {
    #[cfg(windows)]
    {
        let status = service_query_state()?;
        return Ok(match status {
            None => "not-installed".to_string(),
            Some(s) if s == "running" => "running".to_string(),
            Some(s) if s == "stopped" => "stopped".to_string(),
            Some(_) => "unknown".to_string(),
        });
    }

    #[cfg(not(windows))]
    {
        if !crate::core::manager::core_installed().await {
            return Ok("not-installed".to_string());
        }
        if is_mihomo_running().await {
            Ok("running".to_string())
        } else {
            Ok("stopped".to_string())
        }
    }
}

#[tauri::command]
pub async fn test_service_connection() -> Result<bool, String> {
    #[cfg(windows)]
    {
        if service_query_state()? != Some("running".to_string()) {
            return Ok(false);
        }
        let req = crate::service_host::IpcRequest::Ping;
        if send_ipc_request(&req).await.is_err() {
            return Ok(false);
        }
    }
    Ok(is_mihomo_running().await)
}

#[tauri::command]
pub async fn init_service(app: AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    return start_windows_service(&app).await;

    #[cfg(not(windows))]
    {
        use tauri::Emitter;
        if !crate::core::manager::core_installed().await {
            crate::core::manager::install_core().await.map_err(|e| e.to_string())?;
        }
        crate::core::manager::start_core().await.map_err(|e| e.to_string())?;
        let _ = app.emit("core-started", ());
        let _ = app.emit("controled-mihomo-config-updated", ());
        Ok(())
    }
}

#[cfg(windows)]
fn run_elevated_bat(bat_cmd: &str) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_default();
        
    let tmp_bat = std::env::temp_dir().join(format!("nyx_svc_{}.bat", id));
    std::fs::write(&tmp_bat, bat_cmd).map_err(|e| e.to_string())?;

    let ps1_cmd = format!(
        "Start-Process -FilePath '{}' -Verb RunAs -WindowStyle Hidden -Wait",
        tmp_bat.display().to_string().replace('\'', "''")
    );
    let tmp_ps1 = std::env::temp_dir().join(format!("nyx_svc_{}.ps1", id));
    std::fs::write(&tmp_ps1, ps1_cmd).map_err(|e| e.to_string())?;

    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-File",
            &tmp_ps1.to_string_lossy(),
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| e.to_string())?;

    let _ = std::fs::remove_file(&tmp_bat);
    let _ = std::fs::remove_file(&tmp_ps1);

    if !out.status.success() {
        return Err(format!("service install (elevated) failed: {}", output_message(&out)));
    }

    Ok(())
}

#[cfg(windows)]
fn install_service_elevated() -> Result<(), String> {
    let action = match service_query_state()? {
        Some(_) => "config",
        None => "create",
    };
    let bin_path = build_service_binpath()?;

    let bat_cmd = format!(
        "@echo off\r\nchcp 65001 > nul\r\nsc.exe {action} \"{SERVICE_NAME}\" binPath= \"{bin_path}\" start= auto DisplayName= \"{SERVICE_DISPLAY_NAME}\"\r\nif errorlevel 1 exit /b 1\r\nsc.exe start \"{SERVICE_NAME}\"\r\nif errorlevel 1 exit /b 1\r\n",
        action = action,
        SERVICE_NAME = SERVICE_NAME,
        bin_path = bin_path.replace('"', "\\\""),
        SERVICE_DISPLAY_NAME = SERVICE_DISPLAY_NAME
    );

    run_elevated_bat(&bat_cmd)
}

#[tauri::command]
pub async fn install_service() -> Result<(), String> {
    #[cfg(windows)]
    {
        if !crate::commands::window::is_elevated_sync() {
            install_service_elevated()?;
        } else {
            ensure_service_installed_args()?;
            let _ = sc_start_service();
        }
        wait_for_service_state(true).await?;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        Ok(())
    }

    #[cfg(not(windows))]
    {
        crate::core::manager::install_core().await.map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn uninstall_service() -> Result<(), String> {
    #[cfg(windows)]
    {
        crate::core::streaming::stop_streaming();
        if service_query_state()?.is_none() {
            return Ok(());
        }
        if !crate::commands::window::is_elevated_sync() {
            let bat_cmd = format!(
                "@echo off\r\nsc.exe stop \"{SERVICE_NAME}\"\r\nsc.exe delete \"{SERVICE_NAME}\"\r\nif errorlevel 1 exit /b 1\r\n",
                SERVICE_NAME = SERVICE_NAME
            );
            run_elevated_bat(&bat_cmd)?;
        } else {
            sc_stop_service()?;
            wait_for_service_state(false).await?;
            let out = run_sc(&["delete".to_string(), SERVICE_NAME.to_string()])?;
            if !out.status.success() {
                return Err(format!("failed to delete service: {}", output_message(&out)));
            }
        }
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        crate::core::manager::stop_core().await.map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn start_service(app: AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        if service_query_state()?.is_none() {
            return Err("service is not installed".to_string());
        }
        return start_windows_service(&app).await;
    }

    #[cfg(not(windows))]
    {
        use tauri::Emitter;
        crate::core::manager::start_core().await.map_err(|e| e.to_string())?;
        let _ = app.emit("core-started", ());
        let _ = app.emit("controled-mihomo-config-updated", ());
        crate::core::streaming::start_streaming(&app);
        Ok(())
    }
}

#[tauri::command]
pub async fn restart_service(app: AppHandle) -> Result<(), String> {
    crate::core::streaming::stop_streaming();

    #[cfg(windows)]
    {
        if service_query_state()?.is_none() {
            return Err("service is not installed".to_string());
        }
        
        let binary = ensure_core_binary().await?;
        let (config, url) = ensure_runtime_config().await?;
        let work_dir = config.parent().unwrap_or(std::path::Path::new("")).to_string_lossy().into_owned();

        let state = service_query_state()?;
        if state != Some("running".to_string()) {
            if !crate::commands::window::is_elevated_sync() {
                let bat_cmd = format!(
                    "@echo off\r\nchcp 65001 > nul\r\nsc.exe start \"{SERVICE_NAME}\"\r\nif errorlevel 1 exit /b 1\r\n",
                    SERVICE_NAME = SERVICE_NAME
                );
                run_elevated_bat(&bat_cmd)?;
            } else {
                sc_start_service()?;
            }
            wait_for_service_state(true).await?;
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        let req = crate::service_host::IpcRequest::StartCore {
            binary: binary.to_string_lossy().into_owned(),
            work_dir,
            config: config.to_string_lossy().into_owned(),
            max_log_days: read_max_log_days(),
        };

        send_ipc_request(&req).await?;

        sync_controller(&url, &config)?;
        use tauri::Emitter;
        let _ = app.emit("core-started", ());
        let _ = app.emit("controled-mihomo-config-updated", ());
        crate::core::streaming::start_streaming(&app);
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        use tauri::Emitter;
        crate::core::manager::restart_core().await.map_err(|e| e.to_string())?;
        let _ = app.emit("core-started", ());
        let _ = app.emit("controled-mihomo-config-updated", ());
        crate::core::streaming::start_streaming(&app);
        Ok(())
    }
}

#[cfg(windows)]
pub async fn stop_service_for_update() -> Result<(), String> {
    crate::core::streaming::stop_streaming();

    let state = match service_query_state() {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    if state.is_none() {
        return Ok(());
    }

    if state.as_deref() == Some("running") {
        let _ = send_ipc_request(&crate::service_host::IpcRequest::StopCore).await;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        if !crate::commands::window::is_elevated_sync() {
            let bat_cmd = format!(
                "@echo off\r\nsc.exe stop \"{SERVICE_NAME}\"\r\n",
                SERVICE_NAME = SERVICE_NAME
            );
            run_elevated_bat(&bat_cmd)?;
        } else {
            sc_stop_service()?;
        }

        let _ = wait_for_service_state(false).await;
    }

    Ok(())
}

#[tauri::command]
pub async fn stop_service() -> Result<(), String> {
    crate::core::streaming::stop_streaming();

    #[cfg(windows)]
    {
        if service_query_state()?.is_none() {
            return Ok(());
        }
        
        let state = service_query_state()?;
        if state == Some("running".to_string()) {
            let req = crate::service_host::IpcRequest::StopCore;
            let _ = send_ipc_request(&req).await;
        }
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        crate::core::manager::stop_core().await.map_err(|e| e.to_string())
    }
}
