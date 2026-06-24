use std::time::Duration;

use gpui::{App, AsyncApp};

#[cfg(not(target_os = "linux"))]
use rust_i18n::t;

#[cfg(not(target_os = "linux"))]
use tray_icon::{
    menu::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

use crate::app::actions;
use crate::app::state::{AppState, ProxyGroup};

/// Separator embedded in proxy menu ids (`px<US>group<US>node`). U+001F won't
/// occur in proxy names.
const SEP: char = '\u{1f}';

/// Decodes the embedded app icon (PNG) into straight RGBA8 + dimensions.
fn load_icon_rgba() -> Option<(Vec<u8>, u32, u32)> {
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
    Some((rgba, info.width, info.height))
}

#[cfg(not(target_os = "linux"))]
fn load_icon() -> Option<Icon> {
    let (rgba, w, h) = load_icon_rgba()?;
    Icon::from_rgba(rgba, w, h).ok()
}

#[cfg(not(target_os = "linux"))]
fn build_menu(groups: &[ProxyGroup], connected: bool) -> Menu {
    let menu = Menu::new();
    let _ = menu.append(&MenuItem::with_id("show", &t!("tray.show"), true, None));
    let _ = menu.append(&PredefinedMenuItem::separator());

    let toggle_label = if connected {
        t!("tray.disconnect")
    } else {
        t!("tray.connect")
    };
    let _ = menu.append(&MenuItem::with_id(
        "toggle-proxy",
        &toggle_label,
        true,
        None,
    ));
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

#[cfg(not(target_os = "linux"))]
fn build_tray(groups: &[ProxyGroup], connected: bool) -> Option<TrayIcon> {
    let menu = build_menu(groups, connected);
    let mut builder = TrayIconBuilder::new()
        .with_id("nyx")
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

/// Snapshots the tray-relevant slice of app state: proxy groups and whether the
/// proxy is currently connected (TUN on).
fn tray_state(cx: &App) -> (Vec<ProxyGroup>, bool) {
    let st = AppState::global(cx).read(cx);
    (st.groups.clone(), st.tun_enabled)
}

#[cfg(not(target_os = "linux"))]
fn create_icon(cx: &mut App) {
    if cx.has_global::<GlobalTray>() {
        return;
    }
    let (groups, connected) = tray_state(cx);
    if let Some(tray) = build_tray(&groups, connected) {
        cx.set_global(GlobalTray(tray));
    }
}

/// Rebuilds the tray menu from current state
#[cfg(not(target_os = "linux"))]
pub fn rebuild(cx: &App) {
    if let Some(tray) = cx.try_global::<GlobalTray>() {
        let (groups, connected) = tray_state(cx);
        tray.0
            .set_menu(Some(Box::new(build_menu(&groups, connected))));
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
    let (groups, connected) = tray_state(cx);
    linux::rebuild(groups, connected);
}

#[cfg(target_os = "linux")]
pub fn set_enabled(cx: &mut App, enabled: bool) {
    let (groups, connected) = tray_state(cx);
    linux::set_enabled(enabled, groups, connected);
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
        let (groups, connected) = tray_state(cx);
        linux::start(enabled, groups, connected);
    }

    cx.spawn(async move |cx: &mut AsyncApp| {
        #[cfg(not(target_os = "linux"))]
        let menu_rx = MenuEvent::receiver();
        #[cfg(not(target_os = "linux"))]
        let tray_rx = TrayIconEvent::receiver();
        loop {
            cx.background_executor()
                .timer(Duration::from_millis(120))
                .await;
            #[cfg(not(target_os = "linux"))]
            {
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
            #[cfg(target_os = "linux")]
            while let Some(id) = linux::poll_action() {
                cx.update(|cx| handle_menu(&id, cx));
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
        "toggle-proxy" => actions::toggle_tun(cx),
        "restart-core" => actions::restart_core(cx),
        "quit-no-core" => actions::quit_without_core(cx),
        "quit" => actions::quit_with_core(cx),
        _ => {}
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::sync::{Mutex, OnceLock};

    use ksni::menu::{CheckmarkItem, StandardItem, SubMenu};
    use ksni::{Handle, MenuItem, Tray, TrayMethods};
    use rust_i18n::t;

    use super::{load_icon_rgba, SEP};
    use crate::app::runtime;
    use crate::app::state::ProxyGroup;

    static HANDLE: Mutex<Option<Handle<NyxTray>>> = Mutex::new(None);

    /// Persistent action channel: menu callbacks (on the ksni thread) push ids
    /// the gpui loop drains via [`poll_action`]. Lives for the whole process so
    /// it survives tray enable/disable cycles.
    fn actions() -> &'static (Sender<String>, Mutex<Receiver<String>>) {
        static CH: OnceLock<(Sender<String>, Mutex<Receiver<String>>)> = OnceLock::new();
        CH.get_or_init(|| {
            let (tx, rx) = channel();
            (tx, Mutex::new(rx))
        })
    }

    pub fn poll_action() -> Option<String> {
        actions().1.lock().ok()?.try_recv().ok()
    }

    struct NyxTray {
        groups: Vec<ProxyGroup>,
        connected: bool,
        tx: Sender<String>,
    }

    impl Tray for NyxTray {
        fn id(&self) -> String {
            "Nyx".into()
        }
        fn title(&self) -> String {
            "Nyx".into()
        }
        fn icon_pixmap(&self) -> Vec<ksni::Icon> {
            match load_icon_rgba() {
                Some((rgba, w, h)) => {
                    let data = rgba
                        .chunks(4)
                        .flat_map(|p| [p[3], p[0], p[1], p[2]])
                        .collect();
                    vec![ksni::Icon {
                        width: w as i32,
                        height: h as i32,
                        data,
                    }]
                }
                None => Vec::new(),
            }
        }
        fn activate(&mut self, _x: i32, _y: i32) {
            let _ = self.tx.send("show".to_string());
        }
        fn menu(&self) -> Vec<MenuItem<Self>> {
            build_menu(&self.groups, self.connected)
        }
    }

    fn std_item(id: &str, label: String) -> MenuItem<NyxTray> {
        let id = id.to_string();
        StandardItem {
            label,
            activate: Box::new(move |t: &mut NyxTray| {
                let _ = t.tx.send(id.clone());
            }),
            ..Default::default()
        }
        .into()
    }

    fn build_menu(groups: &[ProxyGroup], connected: bool) -> Vec<MenuItem<NyxTray>> {
        let mut items: Vec<MenuItem<NyxTray>> = Vec::new();
        items.push(std_item("show", t!("tray.show").to_string()));
        items.push(MenuItem::Separator);

        let toggle = if connected {
            t!("tray.disconnect")
        } else {
            t!("tray.connect")
        };
        items.push(std_item("toggle-proxy", toggle.to_string()));
        items.push(MenuItem::Separator);

        let selectors: Vec<_> = groups
            .iter()
            .filter(|g| g.kind.as_ref() == "Selector" && !g.all.is_empty())
            .collect();
        if !selectors.is_empty() {
            let mut groups_sub: Vec<MenuItem<NyxTray>> = Vec::new();
            for g in selectors {
                let nodes: Vec<MenuItem<NyxTray>> = g
                    .all
                    .iter()
                    .map(|n| {
                        let id = format!("px{SEP}{}{SEP}{}", g.name, n.name);
                        CheckmarkItem {
                            label: n.name.to_string(),
                            checked: n.name == g.now,
                            activate: Box::new(move |t: &mut NyxTray| {
                                let _ = t.tx.send(id.clone());
                            }),
                            ..Default::default()
                        }
                        .into()
                    })
                    .collect();
                groups_sub.push(
                    SubMenu {
                        label: g.name.to_string(),
                        submenu: nodes,
                        ..Default::default()
                    }
                    .into(),
                );
            }
            items.push(
                SubMenu {
                    label: t!("tray.proxies").to_string(),
                    submenu: groups_sub,
                    ..Default::default()
                }
                .into(),
            );
            items.push(MenuItem::Separator);
        }

        items.push(std_item("mode-rule", t!("tray.modeRule").to_string()));
        items.push(std_item("mode-global", t!("tray.modeGlobal").to_string()));
        items.push(MenuItem::Separator);
        items.push(std_item("restart-core", t!("tray.restartCore").to_string()));
        items.push(std_item("quit-no-core", t!("tray.quitNoCore").to_string()));
        items.push(std_item("quit", t!("tray.quit").to_string()));
        items
    }

    fn spawn_tray(groups: Vec<ProxyGroup>, connected: bool) {
        let tx = actions().0.clone();
        let tray = NyxTray {
            groups,
            connected,
            tx,
        };
        runtime::detach(async move {
            match tray.spawn().await {
                Ok(h) => *HANDLE.lock().unwrap() = Some(h),
                Err(e) => log::error!("[tray] ksni spawn failed: {e}"),
            }
        });
    }

    pub fn start(enabled: bool, groups: Vec<ProxyGroup>, connected: bool) {
        actions();
        if enabled {
            spawn_tray(groups, connected);
        }
    }

    pub fn rebuild(groups: Vec<ProxyGroup>, connected: bool) {
        let handle = HANDLE.lock().unwrap().clone();
        if let Some(h) = handle {
            runtime::detach(async move {
                h.update(move |t: &mut NyxTray| {
                    t.groups = groups;
                    t.connected = connected;
                })
                .await;
            });
        }
    }

    pub fn set_enabled(enabled: bool, groups: Vec<ProxyGroup>, connected: bool) {
        let present = HANDLE.lock().unwrap().is_some();
        if enabled && !present {
            spawn_tray(groups, connected);
        } else if !enabled {
            if let Some(h) = HANDLE.lock().unwrap().take() {
                runtime::detach(async move {
                    h.shutdown().await;
                });
            }
        }
    }
}
