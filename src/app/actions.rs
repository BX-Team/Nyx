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

/// Shows a toast on the main window (in-app notification). No-op until the main
/// window handle has been recorded. Uses `update_window` (not `WindowHandle::update`)
/// so it doesn't lock `Root` — `push_notification` updates `Root` itself.
pub fn notify(note: Notification, cx: &mut App) {
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
        // Run the Win32 show on the next foreground tick, i.e. fully outside the
        // current gpui update/flush — `ShowWindow` pumps WM_* messages back into
        // gpui's window proc, which would otherwise re-enter a live borrow and
        // log "RefCell already borrowed". (`defer` still runs inside the flush.)
        cx.spawn(async move |_cx| crate::app::window::show_now())
            .detach();
    }
    #[cfg(not(windows))]
    {
        if let Some(handle) = cx.try_global::<MainWindow>().map(|m| m.0) {
            let _ = handle.update(cx, |_root, window, _cx| window.activate_window());
        }
    }
}

/// Toggles the main window: hides it if it's already in the foreground,
/// otherwise shows + foregrounds it. Backs the "toggle window" hotkey.
pub fn toggle_window(cx: &mut App) {
    #[cfg(windows)]
    {
        // Run the Win32 calls on the next foreground tick — see `show_window`.
        cx.spawn(async move |_cx| crate::app::window::toggle_now())
            .detach();
    }
    #[cfg(not(windows))]
    {
        cx.activate(true);
        if let Some(handle) = cx.try_global::<MainWindow>().map(|m| m.0) {
            let _ = handle.update(cx, |_root, window, _cx| window.activate_window());
        }
    }
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
    runtime::detach(async move {
        let _ = backend::config::patch_controled_mihomo_config(json!({ "tun": { "enable": new } }))
            .await;
    });
}

/// Flips the system-proxy enable flag in the app config.
pub fn toggle_sysproxy(cx: &mut App) {
    let new = !AppState::global(cx).read(cx).app_flag("sysProxy.enable");
    AppState::global(cx).update(cx, |st, c| {
        if let Some(obj) = st.app_config.as_object_mut() {
            let sp = obj.entry("sysProxy").or_insert_with(|| json!({}));
            if let Some(spo) = sp.as_object_mut() {
                spo.insert("enable".into(), json!(new));
            }
            c.notify();
        }
    });
    runtime::detach(async move {
        let _ = backend::config::patch_app_config(json!({ "sysProxy": { "enable": new } })).await;
    });
}

/// Restarts the mihomo core (fire-and-forget).
pub fn restart_core(_cx: &mut App) {
    runtime::detach(async {
        let _ = backend::manager::restart_core().await;
    });
}

/// Relaunches the app executable, then quits this instance. The relaunch flag
/// tells the new process to wait for this one to release the single-instance
/// lock instead of treating itself as a secondary.
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

/// Stops the core (best-effort, blocking) then quits.
pub fn quit_with_core(cx: &mut App) {
    let _ = runtime::runtime().block_on(backend::manager::stop_core());
    cx.quit();
}
