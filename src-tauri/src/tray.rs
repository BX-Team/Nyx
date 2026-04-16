use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let icon = load_icon(app, true);

    let menu = build_menu(app)?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("Nyx")
        .menu(&menu)
        .on_menu_event(|app, event| handle_menu_event(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                crate::windows::main::toggle_main_window(app);
            }
        })
        .build(app)?;

    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        crate::commands::tray::refresh_tray(&handle).await;
    });

    Ok(())
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let rule = MenuItem::with_id(app, "mode-rule", "Rule Mode", true, None::<&str>)?;
    let global = MenuItem::with_id(app, "mode-global", "Global Mode", true, None::<&str>)?;
    let direct = MenuItem::with_id(app, "mode-direct", "Direct Mode", true, None::<&str>)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let restart = MenuItem::with_id(app, "restart-core", "Restart Core", true, None::<&str>)?;
    let quit_nc = MenuItem::with_id(app, "quit-no-core", "Quit without Core", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    Menu::with_items(
        app,
        &[
            &show, &sep1, &rule, &global, &direct, &sep2, &restart, &quit_nc, &quit,
        ],
    )
}

fn handle_menu_event(app: &AppHandle, id: &str) {
    match id {
        "show" => {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }
        "mode-rule" => {
            tauri::async_runtime::spawn({
                let app = app.clone();
                async move {
                    let _ = crate::core::api::patch_config(serde_json::json!({"mode": "rule"})).await;
                    let _ = app.emit("app-config-updated", ());
                    crate::commands::tray::refresh_tray(&app).await;
                }
            });
        }
        "mode-global" => {
            tauri::async_runtime::spawn({
                let app = app.clone();
                async move {
                    let _ = crate::core::api::patch_config(serde_json::json!({"mode": "global"})).await;
                    let _ = app.emit("app-config-updated", ());
                    crate::commands::tray::refresh_tray(&app).await;
                }
            });
        }
        "mode-direct" => {
            tauri::async_runtime::spawn({
                let app = app.clone();
                async move {
                    let _ = crate::core::api::patch_config(serde_json::json!({"mode": "direct"})).await;
                    let _ = app.emit("app-config-updated", ());
                    crate::commands::tray::refresh_tray(&app).await;
                }
            });
        }
        "restart-core" => {
            tauri::async_runtime::spawn({
                let app = app.clone();
                async move {
                    let _ = crate::commands::core::restart_core(app).await;
                }
            });
        }
        "quit-no-core" => {
            app.exit(0);
        }
        "quit" => {
            tauri::async_runtime::spawn({
                let app = app.clone();
                async move {
                    crate::commands::utils::quit_app(app).await;
                }
            });
        }
        _ => {}
    }
}

pub fn load_icon(_app: &AppHandle, _enabled: bool) -> Image<'static> {
    static ICON: &[u8] = include_bytes!("../icons/icon.png");
    Image::from_bytes(ICON).unwrap_or_else(|_| Image::new(&[], 0, 0))
}

pub fn update_tray_icon(app: &AppHandle, enabled: bool) {
    if let Some(tray) = app.tray_by_id("main") {
        let icon = load_icon(app, enabled);
        let _ = tray.set_icon(Some(icon));
    }
}

pub fn update_tray_tooltip(app: &AppHandle, profile: &str, mode: &str, tun_enabled: bool) {
    let Some(tray) = app.tray_by_id("main") else {
        return;
    };
    let mode_label = match mode {
        "global" => "Global",
        "direct" => "Direct",
        "rule" => "Rule",
        other if !other.is_empty() => other,
        _ => "Rule",
    };
    let profile_line = if profile.is_empty() {
        "Profile: —".to_string()
    } else {
        format!("Profile: {profile}")
    };
    let tun_line = if tun_enabled { "TUN: On" } else { "TUN: Off" };
    let tooltip = format!("Nyx\n{profile_line}\nMode: {mode_label}\n{tun_line}");
    let _ = tray.set_tooltip(Some(&tooltip));
}
