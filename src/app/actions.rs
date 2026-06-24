use gpui::{App, AppContext, Global, WindowHandle};
use gpui_component::{notification::Notification, Root, WindowExt};
use serde_json::json;

use crate::app::runtime;
use crate::app::state::AppState;
use crate::backend;

struct MainWindow(WindowHandle<Root>);
impl Global for MainWindow {}

/// Records the main window handle so actions can show/focus it.
pub fn set_main_window(handle: WindowHandle<Root>, cx: &mut App) {
    cx.set_global(MainWindow(handle));
}

/// Shows a toast on the main window. Uses `update_window` (not
/// `WindowHandle::update`) so it doesn't lock `Root`, which `push_notification`
/// updates itself.
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

/// Brings the main window to the foreground (un-hiding it first if it was
/// closed to the tray).
pub fn show_window(cx: &mut App) {
    cx.activate(true);
    #[cfg(windows)]
    {
        // `spawn` (not `defer`) so the Win32 ShowWindow runs outside this gpui
        // flush — otherwise its WM_* messages re-enter a live borrow and panic.
        cx.spawn(async move |_cx| crate::app::window::show_now())
            .detach();
    }
    #[cfg(not(windows))]
    {
        // The window may have been removed when closed to the tray; recreate it.
        let shown = cx.try_global::<MainWindow>().map(|m| m.0).is_some_and(|h| {
            h.update(cx, |_root, window, _cx| window.activate_window())
                .is_ok()
        });
        if !shown {
            crate::ui::open_main_window(cx, false);
        }
    }
}

/// Toggles the main window: closes it to the tray if open, otherwise recreates
/// and shows it. Backs the "toggle window" hotkey.
pub fn toggle_window(cx: &mut App) {
    #[cfg(windows)]
    {
        // Run the Win32 calls on the next foreground tick — see `show_window`.
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

/// Switches a proxy group's selection (from the tray), then refreshes groups so
/// the UI and tray check-marks update.
pub fn set_proxy(group: String, node: String, cx: &mut App) {
    cx.spawn(async move |cx| {
        let (g, n) = (group.clone(), node.clone());
        let _ = runtime::spawn(async move { backend::mihomo::change_proxy(&g, &n).await }).await;
        crate::app::bootstrap::refresh_runtime_data(cx).await;
    })
    .detach();
}

/// Optimistically switches proxy mode and patches the controlled config.
pub fn set_mode(mode: &'static str, cx: &mut App) {
    AppState::global(cx).update(cx, |st, c| st.set_mode(mode, c));
    runtime::detach(async move {
        let _ = backend::config::patch_controled_mihomo_config(json!({ "mode": mode })).await;
    });
}

/// Flips the TUN main switch (optimistic UI + controlled-config patch).
pub fn toggle_tun(cx: &mut App) {
    let new = !AppState::global(cx).read(cx).tun_enabled;
    AppState::global(cx).update(cx, |st, c| st.set_tun_enabled(new, c));
    crate::app::tray::rebuild(cx);
    runtime::detach(async move {
        let _ = backend::config::patch_controled_mihomo_config(json!({ "tun": { "enable": new } }))
            .await;
        let _ = backend::config::patch_app_config(json!({ "lastConnected": new })).await;
    });
}

/// Sets the system-proxy flag, persists it, and applies/removes the OS proxy.
pub fn set_sysproxy(enable: bool, cx: &mut App) {
    let affect_vpn = AppState::global(cx)
        .read(cx)
        .app_flag("affectVPNConnections");
    AppState::global(cx).update(cx, |st, c| {
        if let Some(obj) = st.app_config.as_object_mut() {
            let sp = obj.entry("sysProxy").or_insert_with(|| json!({}));
            if let Some(spo) = sp.as_object_mut() {
                spo.insert("enable".into(), json!(enable));
            }
            c.notify();
        }
    });
    runtime::detach(async move {
        let _ =
            backend::config::patch_app_config(json!({ "sysProxy": { "enable": enable } })).await;
        backend::sysproxy::apply(enable, affect_vpn).await;
    });
}

/// Flips the system-proxy enable flag (hotkey / tray).
pub fn toggle_sysproxy(cx: &mut App) {
    let new = !AppState::global(cx).read(cx).app_flag("sysProxy.enable");
    set_sysproxy(new, cx);
}

/// Restarts the mihomo core (fire-and-forget).
pub fn restart_core(_cx: &mut App) {
    runtime::detach(async {
        let _ = backend::manager::restart_core().await;
    });
}

/// Relaunches the app executable, then quits this instance. The relaunch flag
/// tells the new process to wait for this one to release the single-instance lock.
pub fn restart_app(cx: &mut App) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe)
            .arg(crate::app::single_instance::RELAUNCH_FLAG)
            .spawn();
    }
    cx.quit();
}

/// Quits, leaving the core/service running.
pub fn quit_without_core(cx: &mut App) {
    cx.quit();
}

/// Clears the OS system proxy, stops the core completely (service-managed or
/// local, best-effort, blocking), then quits.
pub fn quit_with_core(cx: &mut App) {
    backend::sysproxy::clear();
    let _ = runtime::runtime().block_on(backend::service::stop_core_complete());
    cx.quit();
}

/// Ctrl+close: turns the proxy off and clears the system proxy, but leaves the
/// core running in the background.
pub fn disconnect_and_quit(cx: &mut App) {
    backend::sysproxy::clear();
    let _ = runtime::runtime().block_on(backend::config::patch_controled_mihomo_config(
        json!({ "tun": { "enable": false } }),
    ));
    cx.quit();
}
