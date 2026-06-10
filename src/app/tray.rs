use std::time::Duration;

use gpui::{App, AsyncApp, Global};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

use crate::app::actions;

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

/// Builds the tray icon + menu and starts the gpui event-drain loop.
pub fn init(cx: &mut App) {
    let menu = Menu::new();
    let items: [&dyn tray_icon::menu::IsMenuItem; 9] = [
        &MenuItem::with_id("show", "Show Window", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id("mode-rule", "Rule Mode", true, None),
        &MenuItem::with_id("mode-global", "Global Mode", true, None),
        &MenuItem::with_id("mode-direct", "Direct Mode", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id("restart-core", "Restart Core", true, None),
        &MenuItem::with_id("quit-no-core", "Quit without Core", true, None),
        &MenuItem::with_id("quit", "Quit", true, None),
    ];
    if let Err(e) = menu.append_items(&items) {
        log::error!("[tray] menu build failed: {e}");
        return;
    }

    let mut builder = TrayIconBuilder::new()
        .with_tooltip("Nyx")
        .with_menu(Box::new(menu));
    if let Some(icon) = load_icon() {
        builder = builder.with_icon(icon);
    }
    match builder.build() {
        Ok(tray) => cx.set_global(GlobalTray(tray)),
        Err(e) => {
            log::error!("[tray] build failed: {e}");
            return;
        }
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
