use std::time::Duration;

use gpui::{App, AsyncApp};
use tray_icon::{
    menu::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

use rust_i18n::t;

use crate::app::actions;
use crate::app::state::{AppState, ProxyGroup};

/// Separator embedded in proxy menu ids (`px<US>group<US>node`). U+001F won't
/// occur in proxy names.
const SEP: char = '\u{1f}';

/// Decodes the embedded app icon (PNG) into a tray `Icon`.
fn load_icon() -> Option<Icon> {
    static PNG: &[u8] = include_bytes!("../../assets/brand/logo.png");
    let mut reader = png::Decoder::new(std::io::Cursor::new(PNG))
        .read_info()
        .ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
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

/// Builds the tray menu, including a "Proxies" submenu of Selector groups → nodes.
fn build_menu(groups: &[ProxyGroup]) -> Menu {
    let menu = Menu::new();
    let _ = menu.append(&MenuItem::with_id("show", &t!("tray.show"), true, None));
    let _ = menu.append(&PredefinedMenuItem::separator());

    let selectors: Vec<_> = groups
        .iter()
        .filter(|g| g.kind.as_ref() == "Selector" && !g.all.is_empty())
        .collect();
    if !selectors.is_empty() {
        let proxies = Submenu::new(&t!("tray.proxies"), true);
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

    let _ = menu.append(&MenuItem::with_id(
        "mode-rule",
        &t!("tray.modeRule"),
        true,
        None,
    ));
    let _ = menu.append(&MenuItem::with_id(
        "mode-global",
        &t!("tray.modeGlobal"),
        true,
        None,
    ));
    let _ = menu.append(&MenuItem::with_id(
        "mode-direct",
        &t!("tray.modeDirect"),
        true,
        None,
    ));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id(
        "restart-core",
        &t!("tray.restartCore"),
        true,
        None,
    ));
    let _ = menu.append(&MenuItem::with_id(
        "quit-no-core",
        &t!("tray.quitNoCore"),
        true,
        None,
    ));
    let _ = menu.append(&MenuItem::with_id("quit", &t!("tray.quit"), true, None));
    menu
}

/// Builds the tray icon + menu from current proxy groups.
fn build_tray(groups: &[ProxyGroup]) -> Option<TrayIcon> {
    let menu = build_menu(groups);
    let mut builder = TrayIconBuilder::new()
        .with_tooltip("Nyx")
        .with_menu(Box::new(menu));
    if let Some(icon) = load_icon() {
        builder = builder.with_icon(icon);
    }
    match builder.build() {
        Ok(tray) => Some(tray),
        Err(e) => {
            log::error!("[tray] build failed: {e}");
            None
        }
    }
}

/// Keeps the `TrayIcon` alive for the lifetime of the app
#[cfg(not(target_os = "linux"))]
struct GlobalTray(#[allow(dead_code)] TrayIcon);
#[cfg(not(target_os = "linux"))]
impl gpui::Global for GlobalTray {}

#[cfg(not(target_os = "linux"))]
fn create_icon(cx: &mut App) {
    if cx.has_global::<GlobalTray>() {
        return;
    }
    let groups = AppState::global(cx).read(cx).groups.clone();
    if let Some(tray) = build_tray(&groups) {
        cx.set_global(GlobalTray(tray));
    }
}

/// Rebuilds the tray menu from current state
#[cfg(not(target_os = "linux"))]
pub fn rebuild(cx: &App) {
    if let Some(tray) = cx.try_global::<GlobalTray>() {
        let groups = AppState::global(cx).read(cx).groups.clone();
        tray.0.set_menu(Some(Box::new(build_menu(&groups))));
    }
}

/// Adds or removes the tray icon to match the `disableTray` setting at runtime.
#[cfg(not(target_os = "linux"))]
pub fn set_enabled(cx: &mut App, enabled: bool) {
    if enabled {
        create_icon(cx);
    } else if cx.has_global::<GlobalTray>() {
        cx.remove_global::<GlobalTray>();
    }
}

#[cfg(target_os = "linux")]
pub fn rebuild(cx: &App) {
    let groups = AppState::global(cx).read(cx).groups.clone();
    linux::send(linux::TrayCmd::Rebuild(groups));
}

#[cfg(target_os = "linux")]
pub fn set_enabled(cx: &mut App, enabled: bool) {
    let groups = AppState::global(cx).read(cx).groups.clone();
    linux::send(linux::TrayCmd::SetEnabled(enabled, groups));
}

/// Builds the tray icon (unless disabled) and starts the gpui event-drain loop.
pub fn init(cx: &mut App) {
    let enabled = !crate::backend::config::app_config_bool("disableTray");

    #[cfg(not(target_os = "linux"))]
    if enabled {
        create_icon(cx);
    }

    #[cfg(target_os = "linux")]
    {
        let groups = AppState::global(cx).read(cx).groups.clone();
        linux::start(enabled, groups);
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

#[cfg(target_os = "linux")]
mod linux {
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::sync::OnceLock;
    use std::time::Duration;

    use tray_icon::TrayIcon;

    use super::{build_menu, build_tray};
    use crate::app::state::ProxyGroup;

    pub enum TrayCmd {
        Rebuild(Vec<ProxyGroup>),
        SetEnabled(bool, Vec<ProxyGroup>),
    }

    static SENDER: OnceLock<Sender<TrayCmd>> = OnceLock::new();

    pub fn start(enabled: bool, groups: Vec<ProxyGroup>) {
        SENDER.get_or_init(|| {
            let (tx, rx) = channel::<TrayCmd>();
            let _ = std::thread::Builder::new()
                .name("nyx-tray".into())
                .spawn(move || run(rx, enabled, groups));
            tx
        });
    }

    pub fn send(cmd: TrayCmd) {
        if let Some(tx) = SENDER.get() {
            let _ = tx.send(cmd);
        }
    }

    fn run(rx: Receiver<TrayCmd>, enabled: bool, groups: Vec<ProxyGroup>) {
        if let Err(e) = gtk::init() {
            log::error!("[tray] gtk init failed: {e}");
            return;
        }
        let mut tray: Option<TrayIcon> = if enabled { build_tray(&groups) } else { None };
        gtk::glib::timeout_add_local(Duration::from_millis(120), move || {
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    TrayCmd::Rebuild(g) => {
                        if let Some(t) = &tray {
                            t.set_menu(Some(Box::new(build_menu(&g))));
                        }
                    }
                    TrayCmd::SetEnabled(true, g) => {
                        if tray.is_none() {
                            tray = build_tray(&g);
                        }
                    }
                    TrayCmd::SetEnabled(false, _) => tray = None,
                }
            }
            gtk::glib::ControlFlow::Continue
        });
        gtk::main();
    }
}
