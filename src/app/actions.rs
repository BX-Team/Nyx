use gpui::{App, AppContext, Global, WindowHandle};
use gpui_component::{Root, WindowExt, notification::Notification};
use serde_json::json;

use crate::app::runtime;
use crate::app::state::AppState;
use crate::backend;

struct MainWindow(WindowHandle<Root>);
impl Global for MainWindow {}

pub fn set_main_window(handle: WindowHandle<Root>, cx: &mut App) {
    cx.set_global(MainWindow(handle));
}

/// Toasts on the main window via `update_window`, which avoids locking `Root`.
pub fn notify(note: Notification, cx: &mut App) {
    if AppState::global(cx).read(cx).onboarding_active {
        return;
    }
    let Some(handle) = cx.try_global::<MainWindow>().map(|m| m.0) else {
        return;
    };
    let _ = cx.update_window(handle.into(), |_, window, cx| {
        window.push_notification(note, cx);
    });
}

/// Brings the main window up, recreating it if it was closed to the tray.
pub fn show_window(cx: &mut App) {
    cx.activate(true);
    #[cfg(windows)]
    {
        // `spawn`, not `defer`: the Win32 call must land outside this gpui flush,
        // or its WM_* messages re-enter a live borrow and panic.
        cx.spawn(async move |_cx| crate::app::window::show_now())
            .detach();
    }
    #[cfg(not(windows))]
    {
        let shown = cx.try_global::<MainWindow>().map(|m| m.0).is_some_and(|h| {
            h.update(cx, |_root, window, _cx| window.activate_window())
                .is_ok()
        });
        if !shown {
            crate::ui::open_main_window(cx, false);
        }
    }
}

/// Closes the window to the tray if open, otherwise recreates it.
pub fn toggle_window(cx: &mut App) {
    #[cfg(windows)]
    {
        cx.spawn(async move |_cx| crate::app::window::toggle_now())
            .detach();
    }
    #[cfg(not(windows))]
    {
        let closed = cx.try_global::<MainWindow>().map(|m| m.0).is_some_and(|h| {
            h.update(cx, |_root, window, _cx| {
                crate::ui::save_main_window_bounds(window);
                window.remove_window();
            })
            .is_ok()
        });
        if !closed {
            cx.activate(true);
            crate::ui::open_main_window(cx, false);
        }
    }
}

/// Switches a group's selection from the tray, then refreshes so check-marks follow.
pub fn set_proxy(group: String, node: String, cx: &mut App) {
    cx.spawn(async move |cx| {
        let (g, n) = (group.clone(), node.clone());
        let _ = runtime::spawn(async move { backend::mihomo::change_proxy(&g, &n).await }).await;
        crate::app::bootstrap::refresh_runtime_data(cx).await;
    })
    .detach();
}

pub fn set_mode(mode: &'static str, cx: &mut App) {
    AppState::global(cx).update(cx, |st, c| st.set_mode(mode, c));
    runtime::detach(async move {
        let _ = backend::config::patch_controled_mihomo_config(json!({ "mode": mode })).await;
    });
}

pub const MODE_TUN: &str = "tun";
pub const MODE_SYSPROXY: &str = "sysproxy";

pub fn connection_mode(cx: &App) -> &'static str {
    match AppState::global(cx)
        .read(cx)
        .app_config
        .get("connectionMode")
        .and_then(|v| v.as_str())
    {
        Some(MODE_SYSPROXY) => MODE_SYSPROXY,
        _ => MODE_TUN,
    }
}

pub fn connected(cx: &App) -> bool {
    let st = AppState::global(cx).read(cx);
    st.tun_enabled || st.app_flag("sysProxy.enable")
}

pub fn set_connection(mode: &'static str, on: bool, cx: &mut App) {
    let tun = on && mode == MODE_TUN;
    let sysproxy = on && mode == MODE_SYSPROXY;
    let (affect_vpn, running) = {
        let st = AppState::global(cx).read(cx);
        (
            st.app_flag("affectVPNConnections"),
            st.core_status.is_running(),
        )
    };

    AppState::global(cx).update(cx, |st, c| {
        st.set_tun_enabled(tun, c);
        st.set_app_value("sysProxy.enable", json!(sysproxy), c);
        if on {
            st.set_app_value("connectionMode", json!(mode), c);
        }
    });
    crate::app::tray::rebuild(cx);

    cx.spawn(async move |cx: &mut gpui::AsyncApp| {
        let mut app_patch = json!({ "lastConnected": on, "sysProxy": { "enable": sysproxy } });
        if on && let Some(obj) = app_patch.as_object_mut() {
            obj.insert("connectionMode".into(), json!(mode));
        }
        let _ = runtime::spawn(backend::config::patch_app_config(app_patch)).await;

        let patch = if tun {
            json!({ "tun": { "enable": true }, "dns": { "enable": true } })
        } else {
            json!({ "tun": { "enable": false } })
        };
        let _ = runtime::spawn(backend::config::patch_controled_mihomo_config(patch)).await;

        if on && !running {
            // Starting the core also applies the system proxy and refreshes state.
            if !crate::app::bootstrap::start_core_and_streams(cx, tun).await {
                cx.update(|cx| {
                    AppState::global(cx).update(cx, |st, c| {
                        st.set_tun_enabled(false, c);
                        st.set_app_value("sysProxy.enable", json!(false), c);
                    });
                    crate::app::tray::rebuild(cx);
                });
            }
            return;
        }

        let _ = runtime::spawn(backend::sysproxy::apply(sysproxy, affect_vpn)).await;
        crate::app::bootstrap::refresh_runtime_data(cx).await;
    })
    .detach();
}

pub fn toggle_connection(cx: &mut App) {
    let mode = connection_mode(cx);
    let on = !connected(cx);
    set_connection(mode, on, cx);
}

pub fn select_connection_mode(mode: &'static str, cx: &mut App) {
    if connection_mode(cx) == mode {
        return;
    }
    if connected(cx) {
        set_connection(mode, true, cx);
        return;
    }
    AppState::global(cx).update(cx, |st, c| {
        st.set_app_value("connectionMode", json!(mode), c)
    });
    runtime::detach(async move {
        let _ = backend::config::patch_app_config(json!({ "connectionMode": mode })).await;
    });
}

pub fn toggle_tun(cx: &mut App) {
    let on = !AppState::global(cx).read(cx).tun_enabled;
    set_connection(MODE_TUN, on, cx);
}

pub fn set_sysproxy(enable: bool, cx: &mut App) {
    set_connection(MODE_SYSPROXY, enable, cx);
}

pub fn toggle_sysproxy(cx: &mut App) {
    let on = !AppState::global(cx).read(cx).app_flag("sysProxy.enable");
    set_connection(MODE_SYSPROXY, on, cx);
}

pub fn restart_core(_cx: &mut App) {
    runtime::detach(async {
        let _ = backend::core::restart().await;
    });
}

/// The single quit path: un-proxy the machine but leave the service and core up,
/// so reopening restores the previous state without another prompt.
pub fn shutdown_and_quit(cx: &mut App) {
    backend::sysproxy::clear();
    let _ = runtime::runtime().block_on(backend::config::patch_controled_mihomo_config(
        json!({ "tun": { "enable": false } }),
    ));
    cx.quit();
}

/// Relaunches the executable; the flag makes the new process wait for the
/// single-instance lock.
pub fn restart_app(cx: &mut App) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe)
            .arg(crate::app::single_instance::RELAUNCH_FLAG)
            .spawn();
    }
    cx.quit();
}
