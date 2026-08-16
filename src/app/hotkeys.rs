use std::collections::HashMap;
use std::time::Duration;

use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::HotKey};
use gpui::{App, AsyncApp, Global};

use crate::app::actions;
use crate::app::state::AppState;

const BINDINGS: &[(&str, &str)] = &[
    ("showWindowShortcut", "show"),
    ("triggerSysProxyShortcut", "sysproxy"),
    ("triggerTunShortcut", "tun"),
    ("ruleModeShortcut", "rule"),
    ("globalModeShortcut", "global"),
    ("directModeShortcut", "direct"),
    ("restartAppShortcut", "restart-app"),
    ("quitWithoutCoreShortcut", "quit-nc"),
];

struct Hotkeys {
    manager: GlobalHotKeyManager,
    registered: Vec<HotKey>,
    map: HashMap<u32, &'static str>,
}
impl Global for Hotkeys {}

/// Global hotkeys need an X11 connection, so user must use nyx:// deep links in Wayland sessions.
pub fn supported() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::env::var_os("WAYLAND_DISPLAY").is_none()
            && std::env::var("XDG_SESSION_TYPE").as_deref() != Ok("wayland")
    }
    #[cfg(not(target_os = "linux"))]
    {
        true
    }
}

pub fn init(cx: &mut App) {
    if !supported() {
        log::info!("[hotkeys] Wayland session — global hotkeys unavailable, use nyx:// deep links");
        return;
    }
    let manager = match GlobalHotKeyManager::new() {
        Ok(m) => m,
        Err(e) => {
            log::error!("[hotkeys] manager init failed: {e}");
            return;
        }
    };
    cx.set_global(Hotkeys {
        manager,
        registered: Vec::new(),
        map: HashMap::new(),
    });
    reload(cx);

    cx.spawn(async move |cx: &mut AsyncApp| {
        let rx = GlobalHotKeyEvent::receiver();
        loop {
            cx.background_executor()
                .timer(Duration::from_millis(120))
                .await;
            while let Ok(ev) = rx.try_recv() {
                if ev.state == HotKeyState::Pressed {
                    cx.update(|cx| dispatch(ev.id, cx));
                }
            }
        }
    })
    .detach();
}

pub fn reload(cx: &mut App) {
    if cx.try_global::<Hotkeys>().is_none() {
        return;
    }
    let cfg = AppState::global(cx).read(cx).app_config.clone();
    let hk = cx.global_mut::<Hotkeys>();
    if !hk.registered.is_empty() {
        let _ = hk.manager.unregister_all(&hk.registered);
    }
    hk.registered.clear();
    hk.map.clear();
    for (key, action) in BINDINGS {
        let Some(accel) = cfg
            .get(*key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        let hotkey: HotKey = match accel.parse() {
            Ok(h) => h,
            Err(e) => {
                log::warn!("[hotkeys] bad accelerator '{accel}' for {key}: {e}");
                continue;
            }
        };
        if let Err(e) = hk.manager.register(hotkey) {
            log::warn!("[hotkeys] register '{accel}' failed: {e}");
            continue;
        }
        hk.map.insert(hotkey.id(), *action);
        hk.registered.push(hotkey);
    }
}

fn dispatch(id: u32, cx: &mut App) {
    let action = cx
        .try_global::<Hotkeys>()
        .and_then(|hk| hk.map.get(&id).copied());
    match action {
        Some("show") => actions::toggle_window(cx),
        Some("sysproxy") => actions::toggle_sysproxy(cx),
        Some("tun") => actions::toggle_tun(cx),
        Some("rule") => actions::set_mode("rule", cx),
        Some("global") => actions::set_mode("global", cx),
        Some("direct") => actions::set_mode("direct", cx),
        Some("restart-app") => actions::restart_app(cx),
        Some("quit-nc") => actions::shutdown_and_quit(cx),
        _ => {}
    }
}
