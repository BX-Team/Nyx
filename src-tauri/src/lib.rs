#![allow(dead_code)]

mod commands;
mod core;
pub mod service_host;
mod shortcuts;
mod tray;
mod utils;
mod windows;

async fn run_auto_refresh(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let cfg = commands::config::get_profile_config(None)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    let now_ms = chrono::Utc::now().timestamp_millis();
    let items = match cfg["items"].as_array() {
        Some(a) => a.clone(),
        None => return Ok(()),
    };

    for item in items {
        let auto_update = item["autoUpdate"].as_bool().unwrap_or(false);
        if !auto_update {
            continue;
        }
        let interval_minutes = match item["interval"].as_i64() {
            Some(v) if v > 0 => v,
            _ => continue,
        };
        let updated_ms = item["updated"].as_i64().unwrap_or(0);
        let elapsed_minutes = (now_ms - updated_ms) / 60_000;
        if elapsed_minutes < interval_minutes {
            continue;
        }
        let name = item["name"].as_str().unwrap_or("").to_string();
        log::info!("auto-refresh: refreshing profile '{}' (interval={}m, elapsed={}m)", name, interval_minutes, elapsed_minutes);
        if let Err(e) = commands::config::add_profile_item(app.clone(), item.clone()).await {
            log::warn!("auto-refresh: failed to refresh '{}': {}", name, e);
        }
    }
    Ok(())
}

fn read_app_config() -> serde_json::Value {
    let path = utils::dirs::app_config_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_yaml::from_str::<serde_json::Value>(&s).ok())
        .unwrap_or_default()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    log::info!("Nyx starting up");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            use tauri::Manager;
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
                let _ = win.unminimize();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::mihomo::mihomo_version,
            commands::mihomo::mihomo_installed_version,
            commands::mihomo::mihomo_config,
            commands::mihomo::mihomo_close_connection,
            commands::mihomo::mihomo_close_all_connections,
            commands::mihomo::mihomo_rules,
            commands::mihomo::mihomo_proxies,
            commands::mihomo::mihomo_groups,
            commands::mihomo::mihomo_proxy_providers,
            commands::mihomo::mihomo_update_proxy_providers,
            commands::mihomo::mihomo_rule_providers,
            commands::mihomo::mihomo_update_rule_providers,
            commands::mihomo::mihomo_change_proxy,
            commands::mihomo::mihomo_unfixed_proxy,
            commands::mihomo::mihomo_upgrade_geo,
            commands::mihomo::mihomo_upgrade_ui,
            commands::mihomo::mihomo_upgrade,
            commands::mihomo::mihomo_proxy_delay,
            commands::mihomo::mihomo_group_delay,
            commands::mihomo::patch_mihomo_config,
            commands::mihomo::restart_mihomo_connections,
            commands::config::get_app_config,
            commands::config::patch_app_config,
            commands::config::get_controled_mihomo_config,
            commands::config::patch_controled_mihomo_config,
            commands::config::get_profile_config,
            commands::config::set_profile_config,
            commands::config::get_current_profile_item,
            commands::config::get_profile_item,
            commands::config::get_profile_str,
            commands::config::get_file_str,
            commands::config::set_file_str,
            commands::config::get_rule_str,
            commands::config::set_rule_str,
            commands::config::convert_mrs_ruleset,
            commands::config::set_profile_str,
            commands::config::update_profile_item,
            commands::config::change_current_profile,
            commands::config::add_profile_item,
            commands::config::remove_profile_item,
            commands::config::get_runtime_config,
            commands::config::get_runtime_config_str,
            commands::config::get_raw_profile_str,
            commands::config::get_current_profile_str,
            commands::core::restart_core,
            commands::core::start_network_detection,
            commands::core::stop_network_detection,
            commands::core::manual_grant_core_permition,
            commands::core::check_core_permission,
            commands::core::revoke_core_permission,
            commands::service::service_status,
            commands::service::test_service_connection,
            commands::service::init_service,
            commands::service::install_service,
            commands::service::uninstall_service,
            commands::service::start_service,
            commands::service::restart_service,
            commands::service::stop_service,
            commands::sysproxy::trigger_sys_proxy,
            commands::window::window_minimize,
            commands::window::window_maximize,
            commands::window::window_close,
            commands::window::window_is_maximized,
            commands::window::show_main_window,
            commands::window::close_main_window,
            commands::window::trigger_main_window,
            commands::window::set_always_on_top,
            commands::window::is_always_on_top,
            commands::window::set_title_bar_overlay,
            commands::window::needs_first_run_admin,
            commands::window::is_admin,
            commands::window::restart_as_admin,
            commands::window::open_dev_tools,
            commands::tray::show_tray_icon,
            commands::tray::close_tray_icon,
            commands::tray::update_tray_icon,
            commands::tray::set_dock_visible,
            commands::tray::copy_env,
            commands::shortcut::register_shortcut,
            commands::updater::check_update,
            commands::updater::download_and_install_update,
            commands::updater::cancel_update,
            commands::system::check_auto_run,
            commands::system::enable_auto_run,
            commands::system::disable_auto_run,
            commands::system::get_interfaces,
            commands::system::open_uwp_tool,
            commands::system::setup_firewall,
            commands::system::find_system_mihomo,
            commands::system::check_elevate_task,
            commands::system::create_elevate_task,
            commands::system::delete_elevate_task,
            commands::utils::get_version,
            commands::utils::platform,
            commands::utils::get_file_path,
            commands::utils::read_text_file,
            commands::utils::open_file,
            commands::utils::get_user_agent,
            commands::utils::get_app_name,
            commands::utils::get_image_data_url,
            commands::utils::get_icon_data_url,
            commands::utils::alert,
            commands::utils::reset_app_config,
            commands::utils::relaunch_app,
            commands::utils::quit_without_core,
            commands::utils::quit_app,
            commands::utils::not_dialog_quit,
            commands::utils::debug_info,
        ])
        .setup(|app| {
            use tauri::Listener;
            windows::main::create_main_window(&app.handle())?;

            tray::setup_tray(&app.handle())?;

            for evt in ["controled-mihomo-config-updated", "profile-config-updated", "app-config-updated"] {
                let h = app.handle().clone();
                app.listen(evt, move |_| {
                    let h = h.clone();
                    tauri::async_runtime::spawn(async move {
                        commands::tray::refresh_tray(&h).await;
                    });
                });
            }

            {
                use tauri::Manager;
                let app_cfg = read_app_config();
                let silent_start = app_cfg["silentStart"].as_bool().unwrap_or(false);
                let disable_tray = app_cfg["disableTray"].as_bool().unwrap_or(false);
                let always_on_top = app_cfg["alwaysOnTop"].as_bool().unwrap_or(false);

                if let Some(win) = app.get_webview_window("main") {
                    if !silent_start {
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                    if always_on_top {
                        let _ = win.set_always_on_top(true);
                    }
                }

                if disable_tray {
                    if let Some(tray) = app.tray_by_id("main") {
                        let _ = tray.set_visible(false);
                    }
                }

                let shortcut_keys = [
                    "showWindowShortcut",
                    "triggerSysProxyShortcut",
                    "triggerTunShortcut",
                    "ruleModeShortcut",
                    "globalModeShortcut",
                    "directModeShortcut",
                    "restartAppShortcut",
                    "quitWithoutCoreShortcut",
                ];
                for key in shortcut_keys {
                    if let Some(sc) = app_cfg[key].as_str().filter(|s| !s.is_empty()) {
                        if let Err(e) = shortcuts::register_shortcut(app.handle(), None, Some(sc), key) {
                            log::warn!("startup: failed to register shortcut '{}' for '{}': {}", sc, key, e);
                        }
                    }
                }
            }

            let deep_link_handle = app.handle().clone();
            app.listen("deep-link://new-url", move |event| {
                let raw = event.payload().to_string();
                log::info!("deep link raw payload: {}", raw);

                let urls: Vec<String> = serde_json::from_str(&raw).unwrap_or_else(|_| {
                    let s = raw.trim_matches('"').to_string();
                    if s.is_empty() { vec![] } else { vec![s] }
                });

                for url_str in urls {
                    log::info!("deep link url: {}", url_str);
                    if let Ok(parsed) = url::Url::parse(&url_str) {
                        let command = parsed.host_str().unwrap_or(parsed.path().trim_start_matches('/'));
                        if command == "install-config" {
                            let params: std::collections::HashMap<_, _> = parsed.query_pairs().collect();
                            if let Some(config_url) = params.get("url") {
                                let name = params.get("name").map(|n| n.to_string()).unwrap_or_default();
                                let config_url = config_url.to_string();
                                let h = deep_link_handle.clone();
                                tauri::async_runtime::spawn(async move {
                                    let item = serde_json::json!({
                                        "type": "remote",
                                        "url": config_url,
                                        "name": name,
                                    });
                                    match commands::config::add_profile_item(h.clone(), item).await {
                                        Ok(_) => log::info!("profile added via deep link"),
                                        Err(e) => log::error!("deep link add profile failed: {e}"),
                                    }
                                });
                            }
                        }
                    }
                }
            });

            {
                let config_path = utils::dirs::app_config_path();
                if !config_path.exists() {
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
                        "proxyInTray": false,
                        "appTheme": "system",
                        "maxLogDays": 7,
                        "delayTestConcurrency": 50,
                        "disableLoopbackDetector": false,
                        "disableEmbedCA": false,
                        "disableSystemCA": false,
                        "disableNftables": false,
                        "mainSwitchMode": "tun",
                        "sysProxy": {
                            "enable": false,
                            "mode": "manual"
                        },
                        "hosts": [],
                        "safePaths": [],
                        "core": "mihomo",
                        "corePermissionMode": "service"
                    })).unwrap_or_default();
                    let _ = std::fs::write(&config_path, defaults);
                    log::info!("created default app config");
                    use tauri::Emitter;
                    let _ = app.handle().emit("first-run", ());
                }
            }

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let app_cfg = read_app_config();
                let use_service_mode = cfg!(windows)
                    && app_cfg["corePermissionMode"]
                        .as_str()
                        .unwrap_or("service")
                        == "service";

                if use_service_mode {
                    match commands::service::service_status().await {
                        Ok(status) if status == "running" => {
                            use tauri::Emitter;
                            let connected = commands::service::test_service_connection().await
                                .unwrap_or(false);
                            if connected {
                                let _ = handle.emit("core-started", ());
                                core::streaming::start_streaming(&handle);
                            } else {
                                log::warn!("service running but mihomo not reachable, restarting");
                                if let Err(e) = commands::service::start_service(handle.clone()).await {
                                    log::error!("failed to restart service on startup: {e}");
                                    let _ = handle.emit("core-start-failed", e.to_string());
                                }
                            }
                            return;
                        }
                        Ok(status) if status == "stopped" => {
                            if let Err(e) = commands::service::start_service(handle.clone()).await {
                                log::error!("failed to start service mode core: {e}");
                                use tauri::Emitter;
                                let _ = handle.emit("core-start-failed", e.to_string());
                            }
                            return;
                        }
                        Ok(status) if status == "not-installed" => {
                            log::warn!("service mode selected but service is not installed");
                            use tauri::Emitter;
                            let _ = handle.emit(
                                "core-start-failed",
                                "Service mode is enabled, but Nyx service is not installed".to_string(),
                            );
                            return;
                        }
                        Ok(status) => {
                            log::warn!("service mode selected with unexpected status: {status}");
                            use tauri::Emitter;
                            let _ = handle.emit(
                                "core-start-failed",
                                format!("Unexpected service status: {status}"),
                            );
                            return;
                        }
                        Err(e) => {
                            log::error!("failed to query service status: {e}");
                            use tauri::Emitter;
                            let _ = handle.emit("core-start-failed", e.to_string());
                            return;
                        }
                    }
                }

                let selected_core = app_cfg["core"].as_str().unwrap_or("mihomo");
                if let Err(e) = core::manager::install_core_for_core_type(selected_core).await {
                    log::error!("failed to install selected core ({selected_core}): {e}");
                    return;
                }
                use tauri::Emitter;
                match core::manager::start_core().await {
                    Ok(url) => {
                        log::info!("mihomo started at {url}");
                        let _ = handle.emit("core-started", ());
                        core::streaming::start_streaming(&handle);
                    }
                    Err(e) => {
                        log::error!("failed to start mihomo: {e}");
                        let _ = handle.emit("core-start-failed", e.to_string());
                    }
                }
            });

            let refresh_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = run_auto_refresh(&refresh_handle).await {
                    log::warn!("auto-refresh (startup): {e}");
                }
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    if let Err(e) = run_auto_refresh(&refresh_handle).await {
                        log::warn!("auto-refresh error: {e}");
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Nyx");
}
