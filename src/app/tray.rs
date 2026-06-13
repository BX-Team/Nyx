use std::time::Duration;

use gpui::{App, AsyncApp, Global};
use tray_icon::{
    menu::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

use crate::app::actions;
use crate::app::state::AppState;

/// Separator embedded in proxy menu ids (`px<US>group<US>node`). U+001F won't
/// occur in proxy names.
const SEP: char = '\u{1f}';

/// Keeps the `TrayIcon` alive for the lifetime of the app (dropping it removes
/// the icon). Never moved off the main thread.
struct GlobalTray(#[allow(dead_code)] TrayIcon);
impl Global for GlobalTray {}

/// Decodes the embedded app icon (PNG) into a tray `Icon`.
fn load_icon() -> Option<Icon> {
    static PNG: &[u8] = include_bytes!("../../assets/brand/logo.png");
    let mut reader = png::Decoder::new(PNG).read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .chunks(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        _ => return None,
    };
    Icon::from_rgba(rgba, info.width, info.height).ok()
}

/// Builds the full tray menu, including a "Proxies" submenu of switchable
/// (Selector) groups → nodes with the current selection checked.
fn build_menu(cx: &App) -> Menu {
    let menu = Menu::new();
    let _ = menu.append(&MenuItem::with_id("show", "Show Window", true, None));
    let _ = menu.append(&PredefinedMenuItem::separator());

    let groups = AppState::global(cx).read(cx).groups.clone();
    let selectors: Vec<_> = groups
        .iter()
        .filter(|g| g.kind.as_ref() == "Selector" && !g.all.is_empty())
        .collect();
    if !selectors.is_empty() {
        let proxies = Submenu::new("Proxies", true);
        for g in selectors {
            let nodes: Vec<CheckMenuItem> = g
                .all
                .iter()
                .map(|n| {
                    CheckMenuItem::with_id(
                        format!("px{SEP}{}{SEP}{}", g.name, n.name),
                        &n.name,
                        true,
                        n.name == g.now,
                        None,
                    )
                })
                .collect();
            let refs: Vec<&dyn IsMenuItem> = nodes.iter().map(|n| n as &dyn IsMenuItem).collect();
            if let Ok(sub) = Submenu::with_items(&g.name, true, &refs) {
                let _ = proxies.append(&sub);
            }
        }
        let _ = menu.append(&proxies);
        let _ = menu.append(&PredefinedMenuItem::separator());
    }

    let _ = menu.append(&MenuItem::with_id("mode-rule", "Rule Mode", true, None));
    let _ = menu.append(&MenuItem::with_id("mode-global", "Global Mode", true, None));
    let _ = menu.append(&MenuItem::with_id("mode-direct", "Direct Mode", true, None));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id(
        "restart-core",
        "Restart Core",
        true,
        None,
    ));
    let _ = menu.append(&MenuItem::with_id(
        "quit-no-core",
        "Quit without Core",
        true,
        None,
    ));
    let _ = menu.append(&MenuItem::with_id("quit", "Quit", true, None));
    menu
}

/// Builds the tray icon + menu and stores it as a global (idempotent).
fn create_icon(cx: &mut App) {
    if cx.has_global::<GlobalTray>() {
        return;
    }
    let menu = build_menu(cx);
    let mut builder = TrayIconBuilder::new()
        .with_tooltip("Nyx")
        .with_menu(Box::new(menu));
    if let Some(icon) = load_icon() {
        builder = builder.with_icon(icon);
    }
    match builder.build() {
        Ok(tray) => cx.set_global(GlobalTray(tray)),
        Err(e) => log::error!("[tray] build failed: {e}"),
    }
}

/// Rebuilds the tray menu from current state (call when proxy groups change).
pub fn rebuild(cx: &App) {
    if let Some(tray) = cx.try_global::<GlobalTray>() {
        tray.0.set_menu(Some(Box::new(build_menu(cx))));
    }
}

/// Adds or removes the tray icon to match the `disableTray` setting at runtime.
pub fn set_enabled(cx: &mut App, enabled: bool) {
    if enabled {
        create_icon(cx);
    } else if cx.has_global::<GlobalTray>() {
        cx.remove_global::<GlobalTray>();
    }
}

/// Builds the tray icon (unless disabled) and starts the gpui event-drain loop.
pub fn init(cx: &mut App) {
    if !crate::backend::config::app_config_bool("disableTray") {
        create_icon(cx);
    }

    cx.spawn(async move |cx: &mut AsyncApp| {
        let menu_rx = MenuEvent::receiver();
        let tray_rx = TrayIconEvent::receiver();
        loop {
            cx.background_executor()
                .timer(Duration::from_millis(120))
                .await;
            while let Ok(ev) = menu_rx.try_recv() {
                let id = ev.id.0.clone();
                cx.update(|cx| handle_menu(&id, cx));
            }
            while let Ok(ev) = tray_rx.try_recv() {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = ev
                {
                    cx.update(actions::show_window);
                }
            }
        }
    })
    .detach();
}

fn handle_menu(id: &str, cx: &mut App) {
    if let Some(rest) = id.strip_prefix(&format!("px{SEP}")) {
        let mut parts = rest.splitn(2, SEP);
        if let (Some(group), Some(node)) = (parts.next(), parts.next()) {
            actions::set_proxy(group.to_string(), node.to_string(), cx);
        }
        return;
    }
    match id {
        "show" => actions::show_window(cx),
        "mode-rule" => actions::set_mode("rule", cx),
        "mode-global" => actions::set_mode("global", cx),
        "mode-direct" => actions::set_mode("direct", cx),
        "restart-core" => actions::restart_core(cx),
        "quit-no-core" => actions::quit_without_core(cx),
        "quit" => actions::quit_with_core(cx),
        _ => {}
    }
}
