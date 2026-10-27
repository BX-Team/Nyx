use gpui::base::input::{EditorState, InputEvent, InputState, TextareaState};
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, FontWeight, InteractiveElement,
    IntoElement, ParentElement, PathPromptOptions, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window, WindowBounds, WindowOptions, div, px,
    size,
};
use moonglass::theme::ActiveTheme;
use moonglass::toast::ToastKind;
use moonglass::{Button, ButtonSize, IconName, TitleBar, UiText, W_SEMIBOLD, h_flex, v_flex};
use rust_i18n::t;
use std::rc::Rc;

use crate::app::runtime;
use crate::app::state::{AppState, parse_groups};
use crate::backend;

pub(crate) use crate::ui::theme::delay_color;

gpui::actions!(nyx, [Back]);

const KEY_CONTEXT: &str = "Nyx";

pub fn init_keys(cx: &mut App) {
    cx.bind_keys([gpui::KeyBinding::new("escape", Back, Some(KEY_CONTEXT))]);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Route {
    Home,
    Proxies,
    Rules,
    Connections,
    Logs,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogFilter {
    All,
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsSub {
    General,
    Tun,
    SysProxy,
    Dns,
    Mihomo,
    Sniffer,
    Resources,
    Appearance,
    Advanced,
    Shortcuts,
    About,
}

#[derive(Clone)]
pub(crate) struct ProviderRow {
    pub(crate) name: gpui::SharedString,
    pub(crate) subtitle: gpui::SharedString,
}

#[derive(Default)]
pub(crate) struct SubInputs {
    pub(crate) device: Option<Entity<InputState>>,
    pub(crate) mtu: Option<Entity<InputState>>,
    pub(crate) host: Option<Entity<InputState>>,
    pub(crate) bypass: Option<Entity<InputState>>,
    pub(crate) interval: Option<Entity<InputState>>,
    pub(crate) dns_fakeip_range: Option<Entity<InputState>>,
    pub(crate) dns_nameserver: Option<Entity<TextareaState>>,
    pub(crate) dns_default_ns: Option<Entity<TextareaState>>,
    pub(crate) dns_fakeip_filter: Option<Entity<TextareaState>>,
    pub(crate) dns_proxy_ns: Option<Entity<TextareaState>>,
    pub(crate) dns_direct_ns: Option<Entity<TextareaState>>,
    pub(crate) mixed_port: Option<Entity<InputState>>,
    pub(crate) socks_port: Option<Entity<InputState>>,
    pub(crate) http_port: Option<Entity<InputState>>,
    pub(crate) redir_port: Option<Entity<InputState>>,
    pub(crate) tproxy_port: Option<Entity<InputState>>,
    pub(crate) keep_alive_idle: Option<Entity<InputState>>,
    pub(crate) keep_alive_interval: Option<Entity<InputState>>,
    pub(crate) interface_name: Option<Entity<InputState>>,
    pub(crate) skip_auth: Option<Entity<TextareaState>>,
    pub(crate) lan_allowed: Option<Entity<TextareaState>>,
    pub(crate) lan_disallowed: Option<Entity<TextareaState>>,
    pub(crate) sniff_force_domain: Option<Entity<TextareaState>>,
    pub(crate) sniff_skip_domain: Option<Entity<TextareaState>>,
    pub(crate) sniff_skip_dst: Option<Entity<TextareaState>>,
    pub(crate) sniff_skip_src: Option<Entity<TextareaState>>,
}

impl SubInputs {
    fn fields(&self) -> Vec<Entity<InputState>> {
        [
            &self.device,
            &self.mtu,
            &self.host,
            &self.bypass,
            &self.interval,
            &self.dns_fakeip_range,
            &self.mixed_port,
            &self.socks_port,
            &self.http_port,
            &self.redir_port,
            &self.tproxy_port,
            &self.keep_alive_idle,
            &self.keep_alive_interval,
            &self.interface_name,
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect()
    }

    fn areas(&self) -> Vec<Entity<TextareaState>> {
        [
            &self.dns_nameserver,
            &self.dns_default_ns,
            &self.dns_fakeip_filter,
            &self.dns_proxy_ns,
            &self.dns_direct_ns,
            &self.skip_auth,
            &self.lan_allowed,
            &self.lan_disallowed,
            &self.sniff_force_domain,
            &self.sniff_skip_domain,
            &self.sniff_skip_dst,
            &self.sniff_skip_src,
        ]
        .into_iter()
        .flatten()
        .cloned()
        .collect()
    }
}

#[derive(Clone)]
pub(crate) enum EditorTarget {
    Profile { id: String, name: String },
    RuntimeReadonly,
}

/// Smart rule-override editor: custom prepend/append plus the read-only subscription rules.
pub(crate) struct RuleEditState {
    pub(crate) profile_id: String,
    pub(crate) profile_name: String,
    pub(crate) prepend: Vec<String>,
    pub(crate) append: Vec<String>,
    /// Subscription rule strings the user has chosen to drop (override `delete`).
    pub(crate) delete: Vec<String>,
    /// "Add rule" form: type, payload, target policy, insert position.
    pub(crate) kind: SharedString,
    pub(crate) payload: Entity<InputState>,
    pub(crate) policy: SharedString,
    pub(crate) policies: Vec<SharedString>,
    pub(crate) policy_search: Entity<InputState>,
    pub(crate) to_append: bool,
}

type DialogBuild = Rc<dyn Fn(&mut NyxApp, &mut Window, &mut Context<NyxApp>) -> AnyElement>;

/// The open modal: rebuilt from `NyxApp` every frame, so it reads live state.
pub(crate) struct DialogSlot {
    build: DialogBuild,
    width: f32,
    dismissable: bool,
    serial: u64,
    closing: bool,
    focus: FocusHandle,
}

pub(crate) struct NyxApp {
    pub(crate) state: Entity<AppState>,
    pub(crate) route: Route,
    /// First-run welcome flow: `Some(step)` while active (0..=3), `None` once done.
    pub(crate) onboarding_step: Option<u8>,
    /// Where Settings' back row returns to.
    pub(crate) prev_route: Route,
    pub(crate) dialog: Option<DialogSlot>,
    dialog_serial: u64,
    pub(crate) context_menu: Option<moonglass::ContextMenu>,
    context_serial: u64,
    pub(crate) focus: FocusHandle,
    pub(crate) proxies_group: Option<gpui::SharedString>,
    pub(crate) proxies_search: Entity<InputState>,
    pub(crate) proxies_sort_latency: bool,
    pub(crate) proxies_alive_only: bool,
    pub(crate) logs_filter: LogFilter,
    pub(crate) conns_filter: Entity<InputState>,
    pub(crate) conns_detail: Option<gpui::SharedString>,
    /// Connections page tab: `false` = active, `true` = recently closed.
    pub(crate) conns_show_closed: bool,
    pub(crate) logs_scroll: ScrollHandle,
    /// Total log count last rendered — autoscroll fires when it grows.
    pub(crate) logs_seen: std::cell::Cell<usize>,
    pub(crate) settings_sub: SettingsSub,
    pub(crate) sub_inputs: SubInputs,
    /// A field of the open settings page was edited and not saved yet.
    pub(crate) settings_dirty: bool,
    settings_input_subs: Vec<Subscription>,
    pub(crate) recording_shortcut: Option<&'static str>,
    pub(crate) recorder_focus: gpui::FocusHandle,
    pub(crate) service_status: gpui::SharedString,
    pub(crate) service_detail: Option<gpui::SharedString>,
    pub(crate) service_managed: bool,
    pub(crate) core_version_installed: gpui::SharedString,
    pub(crate) service_busy: bool,
    pub(crate) proxy_providers: Vec<ProviderRow>,
    pub(crate) rule_providers: Vec<ProviderRow>,
    pub(crate) resources_busy: bool,
    pub(crate) editor: Option<Entity<EditorState>>,
    pub(crate) editor_target: Option<EditorTarget>,
    pub(crate) rule_editor: Option<RuleEditState>,
    pub(crate) import_url: Entity<InputState>,
    pub(crate) profile_add_local: bool,
    pub(crate) profile_add_name: Entity<InputState>,
    /// Auto-update interval input (hours, remote profiles); empty = off.
    pub(crate) profile_interval: Entity<InputState>,
    pub(crate) profile_add_file: Option<(String, String)>,
    /// Id of the profile being edited; `None` when the modal is creating a new one.
    pub(crate) profile_edit_id: Option<String>,
    pub(crate) profile_add_busy: bool,
    pub(crate) profile_add_error: Option<gpui::SharedString>,
    pub(crate) mrs_input: Option<std::path::PathBuf>,
    pub(crate) mrs_behavior: &'static str,
    pub(crate) connected_since: Option<std::time::Instant>,
    pub(crate) stats_open: bool,
    pub(crate) update_info: Option<backend::updater::UpdateInfo>,
    pub(crate) update_checking: bool,
    pub(crate) update_installing: bool,
    /// Guards the one-time silent auto-check after config loads.
    auto_update_checked: bool,
    _subs: Vec<Subscription>,
}

pub(crate) fn fmt_bytes(n: u64) -> String {
    if n == 0 {
        return "0 B".to_string();
    }
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i <= 1 {
        format!("{:.0} {}", v, UNITS[i])
    } else {
        format!("{:.1} {}", v, UNITS[i])
    }
}

pub(crate) fn fmt_speed(n: u64) -> String {
    format!("{}/s", fmt_bytes(n))
}

/// Parses a providers response into rows, skipping built-in `Compatible` ones.
fn parse_providers(value: &serde_json::Value, is_rule: bool) -> Vec<ProviderRow> {
    let Some(obj) = value.get("providers").and_then(|v| v.as_object()) else {
        return Vec::new();
    };
    let mut out: Vec<ProviderRow> = obj
        .iter()
        .filter_map(|(name, p)| {
            let vehicle = p.get("vehicleType").and_then(|v| v.as_str()).unwrap_or("");
            if vehicle.is_empty() || vehicle == "Compatible" {
                return None;
            }
            let subtitle = if is_rule {
                let behavior = p.get("behavior").and_then(|v| v.as_str()).unwrap_or("");
                let count = p.get("ruleCount").and_then(|v| v.as_u64()).unwrap_or(0);
                format!("{vehicle} · {behavior} · {count}")
            } else {
                let count = p
                    .get("proxies")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                format!("{vehicle} · {count}")
            };
            Some(ProviderRow {
                name: name.clone().into(),
                subtitle: subtitle.into(),
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

impl NyxApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = AppState::global(cx);
        let import_url = cx.new(|cx| InputState::new(window, cx).placeholder("https://…"));
        let profile_add_name = cx.new(|cx| InputState::new(window, cx));
        let profile_interval = cx.new(|cx| InputState::new(window, cx).placeholder("0"));
        let conns_filter = cx.new(|cx| {
            InputState::new(window, cx).placeholder(t!("pages.connections.filter").to_string())
        });
        let proxies_search = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("pages.proxies.searchPlaceholder").to_string())
        });
        let mut subs = vec![
            cx.subscribe(&conns_filter, |_, _, _: &InputEvent, cx| cx.notify()),
            cx.subscribe(&proxies_search, |_, _, _: &InputEvent, cx| cx.notify()),
            cx.subscribe(&import_url, |_, _, _: &InputEvent, cx| cx.notify()),
        ];
        // Re-render on shared-state change; track uptime for the connection bar's timer.
        subs.push(cx.observe(&state, |this: &mut Self, observed, cx| {
            let connected = {
                let st = observed.read(cx);
                st.tun_enabled || st.app_flag("sysProxy.enable")
            };
            if connected && this.connected_since.is_none() {
                this.connected_since = Some(std::time::Instant::now());
            } else if !connected {
                this.connected_since = None;
            }
            if !this.auto_update_checked && !observed.read(cx).app_config.is_null() {
                this.auto_update_checked = true;
                if observed.read(cx).app_flag("autoCheckUpdate") {
                    this.check_update(true, cx);
                }
            }
            cx.notify();
        }));
        subs.push(cx.observe_window_appearance(window, |_, window, cx| {
            crate::ui::theme::apply_saved(Some(window), cx)
        }));

        let onboarding_pending = !backend::config::app_config_bool("onboardingDone");
        state.update(cx, |s, _| s.onboarding_active = onboarding_pending);

        Self {
            state,
            route: Route::Home,
            onboarding_step: if onboarding_pending { Some(0) } else { None },
            prev_route: Route::Home,
            dialog: None,
            dialog_serial: 0,
            context_menu: None,
            context_serial: 0,
            focus: cx.focus_handle(),
            proxies_group: None,
            proxies_search,
            proxies_sort_latency: false,
            proxies_alive_only: false,
            logs_filter: LogFilter::All,
            conns_filter,
            conns_detail: None,
            conns_show_closed: false,
            logs_scroll: ScrollHandle::new(),
            logs_seen: std::cell::Cell::new(0),
            settings_sub: SettingsSub::General,
            sub_inputs: SubInputs::default(),
            settings_dirty: false,
            settings_input_subs: Vec::new(),
            recording_shortcut: None,
            recorder_focus: cx.focus_handle(),
            service_status: SharedString::default(),
            service_detail: None,
            service_managed: backend::core::service_managed(),
            core_version_installed: SharedString::default(),
            service_busy: false,
            proxy_providers: Vec::new(),
            rule_providers: Vec::new(),
            resources_busy: false,
            editor: None,
            editor_target: None,
            rule_editor: None,
            import_url,
            profile_add_local: false,
            profile_add_name,
            profile_interval,
            profile_add_file: None,
            profile_edit_id: None,
            profile_add_busy: false,
            profile_add_error: None,
            mrs_input: None,
            mrs_behavior: "domain",
            connected_since: None,
            stats_open: true,
            update_info: None,
            update_checking: false,
            update_installing: false,
            auto_update_checked: false,
            _subs: subs,
        }
    }

    pub(crate) fn navigate(&mut self, route: Route, window: &mut Window, cx: &mut Context<Self>) {
        self.editor_target = None;
        self.editor = None;
        self.rule_editor = None;
        self.recording_shortcut = None;
        self.conns_detail = None;
        if route == Route::Settings && self.route != Route::Settings {
            self.prev_route = self.route;
            self.route = route;
            self.open_settings_sub(SettingsSub::General, window, cx);
            return;
        }
        self.route = route;
        cx.notify();
    }

    /// Opens a modal rebuilt from `self` every frame; `dismissable` lets Escape and the backdrop close it.
    pub(crate) fn open_dialog(
        &mut self,
        width: f32,
        dismissable: bool,
        build: impl Fn(&mut Self, &mut Window, &mut Context<Self>) -> AnyElement + 'static,
        cx: &mut Context<Self>,
    ) {
        self.dialog_serial += 1;
        self.dialog = Some(DialogSlot {
            build: Rc::new(build),
            width,
            dismissable,
            serial: self.dialog_serial,
            closing: false,
            focus: cx.focus_handle(),
        });
        cx.notify();
    }

    /// Work in a dialog that must not be walked away from: an import or an update install.
    fn dialog_locked(&self) -> bool {
        self.profile_add_busy || self.update_installing
    }

    /// Escape and the mouse's back button: close the topmost thing, else step back a screen.
    pub(crate) fn go_back(&mut self, cx: &mut Context<Self>) {
        if self.context_menu.is_some() {
            self.context_menu = None;
            cx.notify();
        } else if let Some(slot) = &self.dialog {
            if slot.dismissable && !self.dialog_locked() {
                self.close_dialog(cx);
            }
        } else if self.recording_shortcut.is_some() {
            self.recording_shortcut = None;
            cx.notify();
        } else if self.rule_editor.is_some() {
            self.close_rule_editor(cx);
        } else if self.editor_target.is_some() {
            self.close_editor(cx);
        } else if self.conns_detail.is_some() {
            self.conns_detail = None;
            cx.notify();
        } else if self.route == Route::Settings {
            self.close_settings(cx);
        }
    }

    pub(crate) fn close_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(slot) = self.dialog.as_mut() else {
            return;
        };
        if slot.closing {
            return;
        }
        slot.closing = true;
        let serial = slot.serial;
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(moonglass::motion::EXIT)
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.dialog.as_ref().is_some_and(|d| d.serial == serial) {
                    this.dialog = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(crate) fn open_context_menu(
        &mut self,
        position: gpui::Point<gpui::Pixels>,
        entries: Vec<moonglass::MenuEntry>,
        cx: &mut Context<Self>,
    ) {
        self.context_serial += 1;
        self.context_menu = Some(moonglass::ContextMenu {
            position,
            entries,
            serial: self.context_serial,
        });
        cx.notify();
    }

    /// Checks GitHub for a newer release; when not `silent`, also toasts the outcome.
    pub(crate) fn check_update(&mut self, silent: bool, cx: &mut Context<Self>) {
        if self.update_checking || self.update_installing {
            return;
        }
        self.update_checking = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let outcome = match runtime::spawn(backend::updater::check()).await {
                Ok(inner) => inner,
                Err(_) => Err("update check was cancelled".to_string()),
            };
            let _ = this.update(cx, |this, cx| {
                this.update_checking = false;
                match outcome {
                    Ok(Some(info)) => {
                        this.update_info = Some(info);
                        this.open_updater(cx);
                    }
                    Ok(None) if !silent => crate::app::actions::notify(
                        ToastKind::Info,
                        t!("updater.upToDate").to_string(),
                        cx,
                    ),
                    Ok(None) => {}
                    Err(e) if !silent => crate::app::actions::notify(
                        ToastKind::Error,
                        format!("{}: {e}", t!("updater.checkFailed")),
                        cx,
                    ),
                    Err(e) => log::warn!("[updater] auto-check failed: {e}"),
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn install_update(&mut self, cx: &mut Context<Self>) {
        if self.update_installing {
            return;
        }
        self.update_installing = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let outcome = match runtime::spawn(backend::updater::download_and_install()).await {
                Ok(inner) => inner,
                Err(_) => Err("update task was cancelled".to_string()),
            };
            match outcome {
                // `true` → an elevated helper swaps the binary once we quit, then relaunches (Windows).
                Ok(true) => {
                    cx.update(crate::app::actions::shutdown_and_quit);
                }
                Ok(false) => {
                    cx.update(crate::app::actions::restart_app);
                }
                Err(e) => {
                    let _ = this.update(cx, |this, cx| {
                        this.update_installing = false;
                        crate::app::actions::notify(
                            ToastKind::Error,
                            format!("{}: {e}", t!("updater.installFailed")),
                            cx,
                        );
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    pub(crate) fn open_updater(&self, cx: &mut Context<Self>) {
        let view = cx.entity();
        cx.defer(move |cx| {
            view.update(cx, |this, cx| {
                this.open_dialog(
                    460.,
                    true,
                    |this, window, cx| this.render_updater_dialog(window, cx),
                    cx,
                )
            })
        });
    }

    pub(crate) fn open_reset_confirm(&mut self, cx: &mut Context<Self>) {
        self.open_dialog(
            420.,
            true,
            |_, _, cx| {
                let c = cx.theme().colors;
                nyx_dialog(
                    t!("pages.settings.confirmReset").to_string(),
                    v_flex()
                        .gap(px(6.))
                        .child(
                            div()
                                .text_color(c.text2)
                                .child(t!("pages.settings.resetWarning").to_string()),
                        )
                        .child(
                            div()
                                .text_color(moonglass::theme::danger())
                                .child(t!("pages.settings.cannotUndo").to_string()),
                        ),
                    Some(
                        h_flex()
                            .gap(px(8.))
                            .child(
                                Button::new("reset-cancel")
                                    .ghost()
                                    .size(ButtonSize::Md)
                                    .label(t!("common.cancel").to_string())
                                    .on_click(cx.listener(|this, _, _, cx| this.close_dialog(cx))),
                            )
                            .child(
                                Button::new("reset-confirm")
                                    .danger()
                                    .size(ButtonSize::Md)
                                    .label(t!("pages.settings.reset").to_string())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.close_dialog(cx);
                                        this.confirm_reset(cx);
                                    })),
                            )
                            .into_any_element(),
                    ),
                    cx,
                )
            },
            cx,
        );
    }

    pub(crate) fn confirm_reset(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let _ = runtime::spawn(backend::config::reset_app_config()).await;
            cx.update(crate::app::actions::restart_app);
        })
        .detach();
    }

    pub(crate) fn select_language(&mut self, code: &str, cx: &mut Context<Self>) {
        let code = code.to_string();
        self.state.update(cx, |s, c| s.set_language(code, c));
        crate::ui::set_labels(cx);
        crate::app::tray::rebuild(cx);
    }
}

impl NyxApp {
    pub(crate) fn toggle_tun(&mut self, cx: &mut Context<Self>) {
        crate::app::actions::toggle_tun(cx);
    }

    pub(crate) fn toggle_connection(&mut self, cx: &mut Context<Self>) {
        crate::app::actions::toggle_connection(cx);
    }

    pub(crate) fn select_connection_mode(&mut self, mode: &'static str, cx: &mut Context<Self>) {
        crate::app::actions::select_connection_mode(mode, cx);
    }

    pub(crate) fn change_proxy(&mut self, group: String, proxy: String, cx: &mut Context<Self>) {
        self.state.update(cx, |st, c| {
            if let Some(g) = st.groups.iter_mut().find(|g| g.name.as_ref() == group) {
                g.now = proxy.clone().into();
                c.notify();
            }
        });
        cx.spawn(async move |_this, cx| {
            let g = group.clone();
            let p = proxy.clone();
            let _ =
                runtime::spawn(async move { backend::mihomo::change_proxy(&g, &p).await }).await;
            refresh_groups(cx).await;
        })
        .detach();
    }

    pub(crate) fn test_proxy_delay(
        &mut self,
        group: String,
        proxy: String,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |_this, cx| {
            let p = proxy.clone();
            let res =
                runtime::spawn(async move { backend::mihomo::proxy_delay(&p, None).await }).await;
            let delay = match res {
                Ok(Ok(v)) => v
                    .get("delay")
                    .and_then(|d| d.as_u64())
                    .map(|d| d as u32)
                    .filter(|d| *d > 0),
                _ => None,
            };
            cx.update(|cx| {
                AppState::global(cx)
                    .update(cx, |st, c| st.set_node_delay(&group, &proxy, delay, c));
            });
        })
        .detach();
    }

    pub(crate) fn test_group_delay(&mut self, group: String, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let g = group.clone();
            let res =
                runtime::spawn(async move { backend::mihomo::group_delay(&g, None).await }).await;
            if let Ok(Ok(map)) = res {
                cx.update(|cx| {
                    AppState::global(cx).update(cx, |st, c| {
                        if let (Some(obj), Some(grp)) = (
                            map.as_object(),
                            st.groups.iter_mut().find(|x| x.name.as_ref() == group),
                        ) {
                            for node in grp.all.iter_mut() {
                                node.delay = obj
                                    .get(node.name.as_ref())
                                    .and_then(|v| v.as_u64())
                                    .map(|d| d as u32)
                                    .filter(|d| *d > 0);
                            }
                        }
                        c.notify();
                    });
                });
            }
        })
        .detach();
    }

    pub(crate) fn refresh_proxies(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| refresh_groups(cx).await)
            .detach();
    }

    pub(crate) fn refresh_subscription(&mut self, cx: &mut Context<Self>) {
        let current_id = self
            .state
            .read(cx)
            .profiles
            .iter()
            .find(|p| p.is_current && p.kind.as_ref() == "remote")
            .map(|p| p.id.to_string());
        cx.spawn(async move |_this, cx| {
            let Some(id) = current_id else {
                refresh_groups(cx).await;
                return;
            };
            let ok = match runtime::spawn(backend::config::get_profile_item(id)).await {
                Ok(Ok(item)) => {
                    matches!(
                        runtime::spawn(backend::config::add_profile_item(item)).await,
                        Ok(Ok(_))
                    )
                }
                _ => false,
            };
            crate::app::bootstrap::refresh_runtime_data(cx).await;
            let (kind, text) = if ok {
                (ToastKind::Success, t!("pages.home.subUpdated"))
            } else {
                (ToastKind::Error, t!("pages.home.subUpdateFailed"))
            };
            cx.update(|cx| crate::app::actions::notify(kind, text.to_string(), cx));
        })
        .detach();
    }

    /// Clears a group's pinned selection (back to URLTest/Fallback), then re-fetches.
    pub(crate) fn unfix_group(&mut self, group: String, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let g = group.clone();
            let _ = runtime::spawn(async move { backend::api::unfixed_proxy(&g).await }).await;
            refresh_groups(cx).await;
        })
        .detach();
    }

    pub(crate) fn restart_connections(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, _cx| {
            let _ = runtime::spawn(backend::api::restart_connections()).await;
        })
        .detach();
    }

    pub(crate) fn open_mrs_convert(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.mrs_input = None;
        let _ = window;
        self.open_dialog(440., true, Self::render_mrs_dialog, cx);
    }

    pub(crate) fn mrs_set_behavior(&mut self, behavior: &'static str, cx: &mut Context<Self>) {
        self.mrs_behavior = behavior;
        cx.notify();
    }

    pub(crate) fn mrs_pick_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: None,
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let _ = cx.update(|_window, cx| {
                let _ = this.update(cx, |this, cx| {
                    this.mrs_input = Some(path);
                    cx.notify();
                });
            });
        })
        .detach();
    }

    /// Runs `mihomo convert-ruleset` on the picked `.mrs` file, writing the text output beside it.
    pub(crate) fn submit_mrs_convert(&mut self, cx: &mut Context<Self>) {
        let Some(input) = self.mrs_input.clone() else {
            return;
        };
        let behavior = self.mrs_behavior;
        cx.spawn(async move |_this, cx| {
            let p = input.to_string_lossy().to_string();
            let res = runtime::spawn(backend::config::convert_mrs_ruleset(
                p,
                behavior.to_string(),
            ))
            .await;
            let (kind, text) = match res {
                Ok(Ok(content)) => {
                    let stem = input
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "ruleset".to_string());
                    let out = input.with_file_name(format!("{stem}-{behavior}.txt"));
                    match std::fs::write(&out, content) {
                        Ok(_) => (
                            ToastKind::Success,
                            format!("{}: {}", t!("pages.rules.convertDone"), out.display()),
                        ),
                        Err(e) => (ToastKind::Error, e.to_string()),
                    }
                }
                Ok(Err(e)) => (ToastKind::Error, e),
                Err(_) => (
                    ToastKind::Error,
                    t!("pages.rules.convertFailed").to_string(),
                ),
            };
            cx.update(|cx| crate::app::actions::notify(kind, text, cx));
        })
        .detach();
    }
}

async fn refresh_groups(cx: &mut gpui::AsyncApp) {
    if let Ok(Ok(val)) = runtime::spawn(backend::mihomo::groups()).await {
        cx.update(|cx| {
            let parsed = parse_groups(&val);
            AppState::global(cx).update(cx, |st, c| st.set_groups(parsed, c));
        });
    }
}

impl NyxApp {
    fn render_content(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let has_profiles = !self.state.read(cx).profiles.is_empty();
        let route = if !has_profiles && self.route != Route::Home {
            Route::Home
        } else {
            self.route
        };
        if self.rule_editor.is_some() {
            return self.render_rule_editor(window, cx);
        }
        if self.editor_target.is_some() {
            return self.render_editor(cx).into_any_element();
        }
        match route {
            Route::Home => self.render_home(window, cx),
            Route::Proxies => self.render_proxies(window, cx).into_any_element(),
            Route::Rules => self.render_rules(window, cx).into_any_element(),
            Route::Connections => self.render_connections(window, cx).into_any_element(),
            Route::Logs => self.render_logs(window, cx).into_any_element(),
            Route::Settings => div().into_any_element(),
        }
    }

    fn render_updater_dialog(&mut self, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let c = cx.theme().colors;
        let (version, changelog) = self
            .update_info
            .clone()
            .map(|i| (i.version, i.changelog))
            .unwrap_or_default();
        let installing = self.update_installing;
        nyx_dialog(
            t!("updater.versionReady", version => version).to_string(),
            div()
                .id("updater-changelog")
                .max_h(px(380.))
                .overflow_y_scroll()
                .text_color(c.text2)
                .child(gpui::base::text::TextView::markdown(
                    "updater-changelog-md",
                    changelog,
                )),
            Some(
                h_flex()
                    .gap(px(8.))
                    .child(
                        Button::new("updater-later")
                            .ghost()
                            .size(ButtonSize::Md)
                            .label(t!("updater.later").to_string())
                            .disabled(installing)
                            .on_click(cx.listener(|this, _, _, cx| this.close_dialog(cx))),
                    )
                    .child(
                        Button::new("updater-install")
                            .primary()
                            .size(ButtonSize::Md)
                            .icon(IconName::Download)
                            .label(if installing {
                                t!("updater.installing").to_string()
                            } else {
                                t!("updater.update").to_string()
                            })
                            .loading(installing)
                            .on_click(cx.listener(|this, _, _, cx| this.install_update(cx))),
                    )
                    .into_any_element(),
            ),
            cx,
        )
    }

    fn focus_dialog(&self, window: &mut Window) {
        let Some(slot) = self.dialog.as_ref().filter(|d| !d.closing) else {
            return;
        };
        let focus = slot.focus.clone();
        window.on_next_frame(move |window, cx| {
            if !focus.contains_focused(window, cx) {
                focus.focus(window, cx);
            }
        });
    }

    fn render_title_bar(&self, cx: &mut Context<Self>) -> TitleBar {
        let update = self.update_info.as_ref().map(|_| {
            Button::new("title-update")
                .primary()
                .icon(IconName::Download)
                .label(t!("updater.update").to_string())
                .on_click(cx.listener(|this, _, _, cx| this.open_updater(cx)))
        });
        let bar = TitleBar::new("Nyx")
            .logo("brand/logo.png")
            .actions(h_flex().gap(px(6.)).children(update));
        // Route the X through our close logic so it saves bounds; Ctrl+X quits instead.
        #[cfg(not(windows))]
        let bar = bar.on_close(|window, cx| {
            save_main_window_bounds(window);
            if window.modifiers().control {
                crate::app::actions::shutdown_and_quit(cx);
            } else {
                window.remove_window();
            }
        });
        bar
    }
}

/// A modal's content: its title, `body`, and buttons; without `footer` it gets a close button.
pub(crate) fn nyx_dialog(
    title: impl IntoElement,
    body: impl IntoElement,
    footer: Option<AnyElement>,
    cx: &mut Context<NyxApp>,
) -> AnyElement {
    let closable = footer.is_none();
    v_flex()
        .p(px(20.))
        .gap(px(14.))
        .child(
            h_flex()
                .gap(px(8.))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .font_weight(W_SEMIBOLD)
                        .text_size(px(16.))
                        .child(title),
                )
                .when(closable, |row| {
                    row.child(
                        Button::new("dialog-close")
                            .ghost()
                            .size(ButtonSize::Xs)
                            .icon(IconName::Close)
                            .on_click(cx.listener(|this, _, _, cx| this.close_dialog(cx))),
                    )
                }),
        )
        .child(body)
        .children(footer.map(|f| h_flex().justify_end().child(f)))
        .into_any_element()
}

impl Render for NyxApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        moonglass::theme::tick(window, cx);
        let c = cx.theme().colors;
        if window.focused(cx).is_none() {
            self.focus.focus(window, cx);
        }
        self.focus_dialog(window);
        let title_bar = self.render_title_bar(cx);
        let settings = self.route == Route::Settings;
        let has_profiles = !self.state.read(cx).profiles.is_empty();
        let editing = self.editor_target.is_some() || self.rule_editor.is_some();
        let bar =
            (!settings && has_profiles && !editing).then(|| self.render_connection_bar(window, cx));
        let route_key = SharedString::from(format!("route:{:?}", self.route));
        let body = if settings {
            moonglass::motion::enter(
                route_key,
                div().size_full().child(self.render_settings(window, cx)),
                8.,
                window,
                cx,
            )
        } else {
            let content = self.render_content(window, cx);
            h_flex()
                .size_full()
                .items_start()
                .child(self.render_sidebar(window, cx))
                .child(moonglass::motion::enter(
                    route_key,
                    div().flex_1().min_w_0().h_full().child(content),
                    8.,
                    window,
                    cx,
                ))
                .into_any_element()
        };
        let mut dialog = None;
        if let Some(d) = self.dialog.as_ref() {
            let (build, width, dismissable, serial, closing, focus) = (
                d.build.clone(),
                d.width,
                d.dismissable && !self.dialog_locked(),
                d.serial,
                d.closing,
                d.focus.clone(),
            );
            let content = build(self, window, cx);
            let view = cx.entity().downgrade();
            dialog = Some(
                moonglass::modal_frame(
                    serial,
                    px(width),
                    &focus,
                    closing,
                    dismissable,
                    content,
                    move |_, cx| {
                        let _ = view.update(cx, |this, cx| this.close_dialog(cx));
                    },
                    window,
                    cx,
                )
                .into_any_element(),
            );
        }
        let mut context_menu = None;
        if let Some(menu) = self.context_menu.clone() {
            let view = cx.entity().downgrade();
            context_menu = Some(moonglass::context_menu(
                &menu,
                move |_, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.context_menu = None;
                        cx.notify();
                    });
                },
                window,
                cx,
            ));
        }
        let toasts = moonglass::toast::render(if bar.is_some() { 88. } else { 16. }, window, cx);
        let onboarding = self.onboarding_active().then(|| self.render_onboarding(cx));

        v_flex()
            .relative()
            .size_full()
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &Back, _, cx| this.go_back(cx)))
            .bg(c.bg)
            .ui_text(cx)
            .child(title_bar)
            .child(div().flex_1().min_h_0().child(body))
            .children(bar)
            .children(moonglass::chrome::resize_edges(window))
            .when_some(dialog, |root, d| {
                // The dialog is deferred above this layer; without it hover and scroll reach the page.
                root.child(div().absolute().inset_0().occlude()).child(d)
            })
            .child(toasts)
            .children(context_menu)
            .children(onboarding)
            .child(back_button(cx.entity().downgrade()))
    }
}

/// Listens for the mouse's back button anywhere in the window.
fn back_button(view: gpui::WeakEntity<NyxApp>) -> impl IntoElement {
    gpui::canvas(
        |_, _, _| (),
        move |_, _, window, _| {
            window.on_mouse_event(move |e: &gpui::MouseDownEvent, phase, _, cx| {
                if phase == gpui::DispatchPhase::Bubble
                    && e.button == gpui::MouseButton::Navigate(gpui::NavigationDirection::Back)
                {
                    let _ = view.update(cx, |this, cx| this.go_back(cx));
                }
            });
        },
    )
    .absolute()
    .size_0()
}

pub(crate) fn save_main_window_bounds(window: &Window) {
    let b = match window.window_bounds() {
        WindowBounds::Windowed(b) | WindowBounds::Maximized(b) | WindowBounds::Fullscreen(b) => b,
    };
    backend::config::save_window_state(
        b.origin.x.to_f64(),
        b.origin.y.to_f64(),
        b.size.width.to_f64(),
        b.size.height.to_f64(),
    );
}

/// Opens the main window; `silent` hides it to the tray immediately.
pub fn open_main_window(cx: &mut App, silent: bool) {
    let window_bounds = match backend::config::load_window_state() {
        Some((x, y, w, h)) if w >= 400.0 && h >= 300.0 => WindowBounds::Windowed(gpui::Bounds {
            origin: gpui::point(px(x as f32), px(y as f32)),
            size: size(px(w as f32), px(h as f32)),
        }),
        _ => WindowBounds::centered(size(px(1040.0), px(640.0)), cx),
    };
    cx.spawn(async move |cx| {
        cx.update(|cx| {
            let options = WindowOptions {
                window_bounds: Some(window_bounds),
                window_min_size: Some(size(px(860.0), px(560.0))),
                // Wayland app id: matches nyx.desktop so the compositor finds the icon.
                app_id: Some("nyx".to_owned()),
                ..moonglass::chrome::window_options()
            };
            let opened = gpui::open_window(options, cx, |window, cx| {
                window.set_window_title("Nyx");
                cx.new(|cx| NyxApp::new(window, cx))
            });
            let handle = match opened {
                Ok((handle, _)) => handle,
                Err(e) => {
                    log::error!("cannot open the main window: {e}");
                    return;
                }
            };
            crate::app::actions::set_main_window(handle, cx);
            // Close-to-tray; Ctrl+close disconnects and quits.
            let _ = handle.update(cx, |_root, window, cx| {
                crate::app::window::remember(window);
                #[cfg(not(windows))]
                if silent {
                    crate::app::window::hide(window);
                }
                window.on_window_should_close(cx, |window, cx| {
                    save_main_window_bounds(window);
                    if window.modifiers().control {
                        crate::app::actions::shutdown_and_quit(cx);
                        return true;
                    }
                    // `spawn` so the Win32 hide runs outside this borrow (else it re-enters).
                    #[cfg(windows)]
                    {
                        let _ = window;
                        cx.spawn(async move |_cx| crate::app::window::hide_now())
                            .detach();
                        false
                    }
                    // QuitMode::Explicit keeps the app + tray alive after the window closes.
                    #[cfg(not(windows))]
                    {
                        let _ = (window, cx);
                        true
                    }
                });
            });
            #[cfg(windows)]
            if silent {
                cx.spawn(async move |_cx| crate::app::window::hide_now())
                    .detach();
            }
        });
    })
    .detach();
}

impl NyxApp {
    fn refresh_all(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    pub(crate) fn open_profile_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.profile_edit_id = None;
        self.profile_add_local = false;
        self.profile_add_file = None;
        self.import_url
            .update(cx, |s, c| s.set_value("", window, c));
        self.profile_add_name
            .update(cx, |s, c| s.set_value("", window, c));
        self.profile_interval
            .update(cx, |s, c| s.set_value("", window, c));
        self.profile_add_busy = false;
        self.profile_add_error = None;
        self.open_dialog(460., true, Self::render_profile_add_dialog, cx);
    }

    pub(crate) fn open_profile_edit_info(
        &mut self,
        id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.profile_add_file = None;
        self.import_url
            .update(cx, |s, c| s.set_value("", window, c));
        self.profile_add_name
            .update(cx, |s, c| s.set_value("", window, c));
        self.profile_edit_id = Some(id.clone());
        self.profile_add_busy = false;
        self.profile_add_error = None;
        self.open_dialog(460., true, Self::render_profile_add_dialog, cx);
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(item)) = runtime::spawn(backend::config::get_profile_item(id)).await else {
                return;
            };
            let is_local = item["type"].as_str() == Some("local");
            let url = item["url"].as_str().unwrap_or_default().to_string();
            let name = item["name"].as_str().unwrap_or_default().to_string();
            let interval_min = item["interval"].as_i64().unwrap_or(0);
            let hours = if interval_min > 0 {
                (interval_min / 60).to_string()
            } else {
                String::new()
            };
            let _ = cx.update(|window, cx| {
                let _ = this.update(cx, |this, cx| {
                    this.profile_add_local = is_local;
                    this.import_url
                        .update(cx, |s, c| s.set_value(url.clone(), window, c));
                    this.profile_add_name
                        .update(cx, |s, c| s.set_value(name.clone(), window, c));
                    this.profile_interval
                        .update(cx, |s, c| s.set_value(hours.clone(), window, c));
                    cx.notify();
                });
            });
        })
        .detach();
    }

    pub(crate) fn close_profile_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.profile_add_busy {
            return;
        }
        let _ = window;
        self.profile_edit_id = None;
        self.profile_add_error = None;
        self.close_dialog(cx);
    }

    pub(crate) fn profile_add_set_local(&mut self, local: bool, cx: &mut Context<Self>) {
        self.profile_add_local = local;
        cx.notify();
    }

    /// Picks a local YAML file for the modal (stores name + contents, doesn't import yet).
    pub(crate) fn profile_add_pick_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = window;
        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: None,
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = rx.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "profile".to_string());
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("[profile_add] read failed: {e}");
                    return;
                }
            };
            let _ = cx.update(|window, cx| {
                let _ = this.update(cx, |this, cx| {
                    this.profile_add_local = true;
                    this.profile_add_file = Some((name.clone(), content));
                    if this.profile_add_name.read(cx).value().trim().is_empty() {
                        this.profile_add_name
                            .update(cx, |s, c| s.set_value(name.clone(), window, c));
                    }
                    cx.notify();
                });
            });
        })
        .detach();
    }

    pub(crate) fn submit_profile_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.profile_add_name.read(cx).value().trim().to_string();
        let edit_id = self.profile_edit_id.clone();
        let item = if self.profile_add_local {
            match self.profile_add_file.clone() {
                Some((fname, content)) => {
                    let name = if name.is_empty() { fname } else { name };
                    let mut v =
                        serde_json::json!({ "type": "local", "name": name, "file": content });
                    if let Some(id) = &edit_id {
                        v["id"] = serde_json::Value::String(id.clone());
                    }
                    v
                }
                // Editing a local profile without re-picking a file: rename only.
                None => {
                    let Some(id) = edit_id else { return };
                    self.profile_add_busy = true;
                    self.profile_add_error = None;
                    cx.notify();
                    cx.spawn_in(window, async move |this, cx| {
                        let res = runtime::spawn(async move {
                            let mut existing = backend::config::get_profile_item(id).await?;
                            existing["name"] = serde_json::Value::String(name);
                            backend::config::update_profile_item(existing).await
                        })
                        .await;
                        let err = match res {
                            Ok(Ok(())) => None,
                            Ok(Err(e)) => Some(e),
                            Err(_) => Some("task cancelled".to_string()),
                        };
                        Self::finish_profile_add(this, cx, err).await;
                    })
                    .detach();
                    return;
                }
            }
        } else {
            let url = self.import_url.read(cx).value().trim().to_string();
            if url.is_empty() {
                return;
            }
            let interval_min = self
                .profile_interval
                .read(cx)
                .value()
                .trim()
                .parse::<i64>()
                .map(|h| h.max(0) * 60)
                .unwrap_or(0);
            let mut v = serde_json::json!({
                "type": "remote", "url": url, "name": name, "interval": interval_min,
            });
            if let Some(id) = &edit_id {
                v["id"] = serde_json::Value::String(id.clone());
            }
            v
        };
        self.profile_add_busy = true;
        self.profile_add_error = None;
        cx.notify();
        cx.spawn_in(window, async move |this, cx| {
            let added = runtime::spawn(backend::config::add_profile_item(item)).await;
            let err = match added {
                Ok(Ok(_)) => None,
                Ok(Err(e)) => Some(e),
                Err(_) => Some("task cancelled".to_string()),
            };
            Self::finish_profile_add(this, cx, err).await;
        })
        .detach();
    }

    async fn finish_profile_add(
        this: gpui::WeakEntity<Self>,
        cx: &mut gpui::AsyncWindowContext,
        err: Option<String>,
    ) {
        if err.is_none() {
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        }
        let _ = this.update_in(cx, |this, window, cx| {
            this.profile_add_busy = false;
            match err {
                None => this.close_profile_add(window, cx),
                Some(e) => {
                    log::warn!("[profile] import failed: {e}");
                    this.profile_add_error = Some(e.into());
                    cx.notify();
                }
            }
        });
    }

    pub(crate) fn activate_profile(&mut self, id: String, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let _ = runtime::spawn(backend::config::change_current_profile(id)).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    pub(crate) fn delete_profile(&mut self, id: String, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let _ = runtime::spawn(backend::config::remove_profile_item(id)).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    pub(crate) fn update_profile(&mut self, id: String, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            if let Ok(Ok(item)) = runtime::spawn(backend::config::get_profile_item(id)).await {
                let _ = runtime::spawn(backend::config::add_profile_item(item)).await;
            }
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    pub(crate) fn update_all_profiles(&mut self, cx: &mut Context<Self>) {
        let ids: Vec<String> = self
            .state
            .read(cx)
            .profiles
            .iter()
            .filter(|p| p.kind.as_ref() == "remote")
            .map(|p| p.id.to_string())
            .collect();
        cx.spawn(async move |_this, cx| {
            for id in ids {
                if let Ok(Ok(item)) = runtime::spawn(backend::config::get_profile_item(id)).await {
                    let _ = runtime::spawn(backend::config::add_profile_item(item)).await;
                }
            }
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }
}

impl NyxApp {
    pub(crate) fn open_profile_editor(
        &mut self,
        id: String,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let editor = moonglass::code::editor(moonglass::code::Flavor::Yaml, "", false, window, cx);
        self.editor = Some(editor.clone());
        self.editor_target = Some(EditorTarget::Profile {
            id: id.clone(),
            name,
        });
        cx.notify();
        cx.spawn_in(window, async move |_this, cx| {
            if let Ok(Ok(text)) = runtime::spawn(backend::config::get_profile_str(id)).await {
                let _ = cx.update(|window, cx| {
                    editor.update(cx, |st, c| st.set_value(text, window, c));
                });
            }
        })
        .detach();
    }

    pub(crate) fn save_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.editor.clone() else {
            return;
        };
        let Some(target) = self.editor_target.clone() else {
            return;
        };
        let text = editor.read(cx).value().to_string();
        cx.spawn(async move |_this, cx| {
            match target {
                EditorTarget::Profile { id, .. } => {
                    let _ = runtime::spawn(backend::config::set_profile_str(id, text)).await;
                    let _ = runtime::spawn(backend::config::reload_current_profile()).await;
                }
                EditorTarget::RuntimeReadonly => {}
            }
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    pub(crate) fn close_editor(&mut self, cx: &mut Context<Self>) {
        self.editor = None;
        self.editor_target = None;
        cx.notify();
    }
}

/// Parses an override file's `prepend` / `append` / `delete` string lists.
fn parse_rule_overrides(text: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
    let val: serde_yaml::Value = serde_yaml::from_str(text).unwrap_or(serde_yaml::Value::Null);
    let list = |key: &str| -> Vec<String> {
        val.get(key)
            .and_then(|v| v.as_sequence())
            .map(|s| {
                s.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    };
    (list("prepend"), list("append"), list("delete"))
}

impl NyxApp {
    pub(crate) fn open_rule_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let prof = {
            let st = self.state.read(cx);
            st.profiles.iter().find(|p| p.is_current).cloned()
        };
        let Some(prof) = prof else {
            return;
        };
        let id = prof.id.to_string();
        let name = prof.name.to_string();

        let first = crate::ui::pages::RULE_TYPES[0];
        let payload = cx.new(|cx| {
            InputState::new(window, cx).placeholder(crate::ui::pages::rule_example(first))
        });
        let policy_search =
            cx.new(|cx| InputState::new(window, cx).placeholder(t!("common.search").to_string()));

        // Policy list: live groups + nodes, then built-in policies; default DIRECT.
        let mut policies: Vec<SharedString> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        {
            let st = self.state.read(cx);
            for g in &st.groups {
                if seen.insert(g.name.to_string()) {
                    policies.push(g.name.clone());
                }
            }
            for g in &st.groups {
                for node in &g.all {
                    if seen.insert(node.name.to_string()) {
                        policies.push(node.name.clone());
                    }
                }
            }
        }
        for p in ["DIRECT", "REJECT", "REJECT-DROP", "PASS", "COMPATIBLE"] {
            if seen.insert(p.to_string()) {
                policies.push(p.into());
            }
        }

        self.rule_editor = Some(RuleEditState {
            profile_id: id.clone(),
            profile_name: name,
            prepend: Vec::new(),
            append: Vec::new(),
            delete: Vec::new(),
            kind: first.into(),
            payload,
            policy: "DIRECT".into(),
            policies,
            policy_search,
            to_append: true,
        });
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let text = match runtime::spawn(backend::config::get_rule_str(id)).await {
                Ok(Ok(t)) => t,
                _ => String::new(),
            };
            let (prepend, append, delete) = parse_rule_overrides(&text);
            let _ = this.update(cx, |this, cx| {
                if let Some(re) = this.rule_editor.as_mut() {
                    re.prepend = prepend;
                    re.append = append;
                    re.delete = delete;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(crate) fn rule_editor_set_append(&mut self, to_append: bool, cx: &mut Context<Self>) {
        if let Some(re) = self.rule_editor.as_mut() {
            re.to_append = to_append;
            cx.notify();
        }
    }

    pub(crate) fn rule_editor_set_kind(
        &mut self,
        kind: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(re) = self.rule_editor.as_mut() {
            let example = crate::ui::pages::rule_example(kind.as_ref());
            re.payload
                .update(cx, |s, c| s.set_placeholder(example, window, c));
            re.kind = kind;
            cx.notify();
        }
    }

    pub(crate) fn rule_editor_set_policy(&mut self, policy: SharedString, cx: &mut Context<Self>) {
        if let Some(re) = self.rule_editor.as_mut() {
            re.policy = policy;
            cx.notify();
        }
    }

    pub(crate) fn rule_editor_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(re) = self.rule_editor.as_ref() else {
            return;
        };
        let kind = re.kind.to_string();
        let payload = re.payload.read(cx).value().trim().to_string();
        let policy = re.policy.to_string();
        if policy.is_empty() {
            return;
        }
        // MATCH takes no payload; everything else needs one.
        let rule = if kind == "MATCH" {
            format!("MATCH,{policy}")
        } else if payload.is_empty() {
            return;
        } else {
            format!("{kind},{payload},{policy}")
        };
        let to_append = re.to_append;
        let payload_input = re.payload.clone();
        if let Some(re) = self.rule_editor.as_mut() {
            if to_append {
                re.append.push(rule);
            } else {
                re.prepend.insert(0, rule);
            }
        }
        payload_input.update(cx, |s, c| s.set_value("", window, c));
        cx.notify();
    }

    pub(crate) fn rule_editor_remove(&mut self, append: bool, idx: usize, cx: &mut Context<Self>) {
        if let Some(re) = self.rule_editor.as_mut() {
            let list = if append {
                &mut re.append
            } else {
                &mut re.prepend
            };
            if idx < list.len() {
                list.remove(idx);
                cx.notify();
            }
        }
    }

    pub(crate) fn rule_editor_toggle_delete(&mut self, rule: String, cx: &mut Context<Self>) {
        if let Some(re) = self.rule_editor.as_mut() {
            if let Some(pos) = re.delete.iter().position(|r| r == &rule) {
                re.delete.remove(pos);
            } else {
                re.delete.push(rule);
            }
            cx.notify();
        }
    }

    pub(crate) fn save_rule_editor(&mut self, cx: &mut Context<Self>) {
        let Some(re) = self.rule_editor.as_ref() else {
            return;
        };
        let id = re.profile_id.clone();
        let doc = serde_json::json!({
            "prepend": re.prepend,
            "append": re.append,
            "delete": re.delete,
        });
        let text = serde_yaml::to_string(&doc).unwrap_or_default();
        self.rule_editor = None;
        cx.notify();
        cx.spawn(async move |_this, cx| {
            let _ = runtime::spawn(backend::config::set_rule_str(id, text)).await;
            let _ = runtime::spawn(backend::config::reload_current_profile()).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    pub(crate) fn close_rule_editor(&mut self, cx: &mut Context<Self>) {
        self.rule_editor = None;
        cx.notify();
    }
}

impl NyxApp {
    fn render_editor(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let c = cx.theme().colors;
        let (title, readonly) = match &self.editor_target {
            Some(EditorTarget::Profile { name, .. }) => (name.clone(), false),
            Some(EditorTarget::RuntimeReadonly) => ("Runtime config".to_string(), true),
            None => (String::new(), true),
        };
        let header = h_flex()
            .flex_none()
            .gap(px(12.))
            .px(px(18.))
            .py(px(14.))
            .border_b_1()
            .border_color(c.border)
            .child(
                Button::new("editor-back")
                    .ghost()
                    .icon(IconName::ArrowLeft)
                    .tooltip(t!("common.back").to_string())
                    .on_click(cx.listener(|this, _, _, cx| this.close_editor(cx))),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .font_weight(FontWeight::BOLD)
                    .text_size(px(17.))
                    .child(title),
            )
            .when(!readonly, |row| {
                row.child(
                    Button::new("editor-save")
                        .primary()
                        .label(t!("common.save").to_string())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.save_editor(cx);
                            crate::app::actions::notify(
                                ToastKind::Success,
                                t!("common.saved").to_string(),
                                cx,
                            );
                        })),
                )
            });
        let body = match self.editor.clone() {
            Some(state) => div()
                .flex_1()
                .min_h_0()
                .child(moonglass::code::CodeEditor::new(&state))
                .into_any_element(),
            None => div().into_any_element(),
        };
        v_flex().size_full().child(header).child(body)
    }
}

impl NyxApp {
    pub(crate) fn set_proxy_mode(&mut self, mode: &str, cx: &mut Context<Self>) {
        let mode = mode.to_string();
        self.state.update(cx, |st, c| st.set_mode(mode.clone(), c));
        cx.spawn(async move |_this, cx| {
            let patch = serde_json::json!({ "mode": mode });
            let _ = runtime::spawn(backend::config::patch_controled_mihomo_config(patch)).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    pub(crate) fn toggle_stats(&mut self, cx: &mut Context<Self>) {
        self.stats_open = !self.stats_open;
        cx.notify();
    }

    pub(crate) fn set_app_flag(&mut self, patch: serde_json::Value, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let _ = runtime::spawn(backend::config::patch_app_config(patch)).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    pub(crate) fn open_settings_sub(
        &mut self,
        sub: SettingsSub,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let st = self.state.read(cx);
        let mut inputs = SubInputs::default();
        let mk = |window: &mut Window, cx: &mut Context<Self>, val: String, ph: &'static str| {
            cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(val)
                    .placeholder(ph)
            })
        };
        match sub {
            SettingsSub::Tun => {
                let device = st
                    .ctl("tun.device")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Nyx")
                    .to_string();
                let mtu = st
                    .ctl("tun.mtu")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1500)
                    .to_string();
                inputs.device = Some(mk(window, cx, device, "Nyx"));
                inputs.mtu = Some(mk(window, cx, mtu, "1500"));
            }
            SettingsSub::SysProxy => {
                let host = st
                    .app_config
                    .get("sysProxy")
                    .and_then(|s| s.get("host"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let bypass = st
                    .app_config
                    .get("sysProxy")
                    .and_then(|s| s.get("bypass"))
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();
                inputs.host = Some(mk(window, cx, host, "127.0.0.1:7890"));
                inputs.bypass = Some(mk(window, cx, bypass, "localhost, 127.*"));
            }
            SettingsSub::Advanced => {
                let interval = st
                    .app_config
                    .get("networkDetectionInterval")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10)
                    .to_string();
                inputs.interval = Some(mk(window, cx, interval, "10"));
            }
            SettingsSub::Dns => {
                // Compute owned values first so the `st` borrow ends before `cx.new` below.
                let join_arr = |path: &str| -> String {
                    st.ctl(path)
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|x| x.as_str())
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                        .unwrap_or_default()
                };
                let range = st
                    .ctl("dns.fake-ip-range")
                    .and_then(|v| v.as_str())
                    .unwrap_or("198.18.0.1/16")
                    .to_string();
                let nameserver = join_arr("dns.nameserver");
                let default_ns = join_arr("dns.default-nameserver");
                let fakeip_filter = join_arr("dns.fake-ip-filter");
                let proxy_ns = join_arr("dns.proxy-server-nameserver");
                let direct_ns = join_arr("dns.direct-nameserver");

                let mk_multi =
                    |window: &mut Window, cx: &mut Context<Self>, val: String, ph: &'static str| {
                        cx.new(|cx| {
                            TextareaState::new(window, cx)
                                .auto_grow(2, 6)
                                .default_value(val)
                                .placeholder(ph)
                        })
                    };
                inputs.dns_fakeip_range = Some(mk(window, cx, range, "198.18.0.1/16"));
                inputs.dns_nameserver = Some(mk_multi(
                    window,
                    cx,
                    nameserver,
                    "https://doh.pub/dns-query",
                ));
                inputs.dns_default_ns = Some(mk_multi(window, cx, default_ns, "tls://223.5.5.5"));
                inputs.dns_fakeip_filter = Some(mk_multi(window, cx, fakeip_filter, "*.lan"));
                inputs.dns_proxy_ns =
                    Some(mk_multi(window, cx, proxy_ns, "https://doh.pub/dns-query"));
                inputs.dns_direct_ns = Some(mk_multi(window, cx, direct_ns, "system"));
            }
            SettingsSub::Mihomo => {
                let port = |path: &str, default: u64| -> String {
                    st.ctl(path)
                        .and_then(|v| v.as_u64())
                        .unwrap_or(default)
                        .to_string()
                };
                let join_arr = |path: &str| -> String {
                    st.ctl(path)
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|x| x.as_str())
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                        .unwrap_or_default()
                };
                let mixed = port("mixed-port", 7897);
                let socks = port("socks-port", 0);
                let http = port("port", 0);
                let redir = port("redir-port", 0);
                let tproxy = port("tproxy-port", 0);
                let idle = port("keep-alive-idle", 15);
                let interval = port("keep-alive-interval", 15);
                let iface = st
                    .ctl("interface-name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let skip_auth = join_arr("skip-auth-prefixes");
                let lan_allowed = join_arr("lan-allowed-ips");
                let lan_disallowed = join_arr("lan-disallowed-ips");

                let mk_multi =
                    |window: &mut Window, cx: &mut Context<Self>, val: String, ph: &'static str| {
                        cx.new(|cx| {
                            TextareaState::new(window, cx)
                                .auto_grow(2, 6)
                                .default_value(val)
                                .placeholder(ph)
                        })
                    };
                inputs.mixed_port = Some(mk(window, cx, mixed, "7897"));
                inputs.socks_port = Some(mk(window, cx, socks, "0"));
                inputs.http_port = Some(mk(window, cx, http, "0"));
                inputs.redir_port = Some(mk(window, cx, redir, "0"));
                inputs.tproxy_port = Some(mk(window, cx, tproxy, "0"));
                inputs.keep_alive_idle = Some(mk(window, cx, idle, "15"));
                inputs.keep_alive_interval = Some(mk(window, cx, interval, "15"));
                inputs.interface_name = Some(mk(window, cx, iface, "auto"));
                inputs.skip_auth = Some(mk_multi(window, cx, skip_auth, "127.0.0.1/32"));
                inputs.lan_allowed = Some(mk_multi(window, cx, lan_allowed, "0.0.0.0/0"));
                inputs.lan_disallowed =
                    Some(mk_multi(window, cx, lan_disallowed, "192.168.0.0/16"));
            }
            SettingsSub::Sniffer => {
                let join_arr = |path: &str| -> String {
                    st.ctl(path)
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|x| x.as_str())
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                        .unwrap_or_default()
                };
                let force_domain = join_arr("sniffer.force-domain");
                let skip_domain = join_arr("sniffer.skip-domain");
                let skip_dst = join_arr("sniffer.skip-dst-address");
                let skip_src = join_arr("sniffer.skip-src-address");
                let mk_multi =
                    |window: &mut Window, cx: &mut Context<Self>, val: String, ph: &'static str| {
                        cx.new(|cx| {
                            TextareaState::new(window, cx)
                                .auto_grow(2, 6)
                                .default_value(val)
                                .placeholder(ph)
                        })
                    };
                inputs.sniff_force_domain =
                    Some(mk_multi(window, cx, force_domain, "+.example.com"));
                inputs.sniff_skip_domain =
                    Some(mk_multi(window, cx, skip_domain, "+.push.apple.com"));
                inputs.sniff_skip_dst = Some(mk_multi(window, cx, skip_dst, "192.168.0.0/16"));
                inputs.sniff_skip_src = Some(mk_multi(window, cx, skip_src, "192.168.0.0/16"));
            }
            _ => {}
        }
        let mut subs = Vec::new();
        for field in inputs.fields() {
            subs.push(cx.subscribe(&field, |this, _, ev: &InputEvent, cx| {
                if matches!(ev, InputEvent::Change) {
                    this.settings_dirty = true;
                    cx.notify();
                }
            }));
        }
        for area in inputs.areas() {
            subs.push(cx.subscribe(&area, |this, _, ev: &InputEvent, cx| {
                if matches!(ev, InputEvent::Change) {
                    this.settings_dirty = true;
                    cx.notify();
                }
            }));
        }
        self.settings_input_subs = subs;
        self.settings_dirty = false;
        self.sub_inputs = inputs;
        self.settings_sub = sub;
        self.recording_shortcut = None;
        // Keep what was shown last while the refresh runs, so reopening a page does not blink.
        if matches!(sub, SettingsSub::Mihomo) {
            self.refresh_service_info(cx);
        }
        if matches!(sub, SettingsSub::Resources) {
            self.refresh_providers(cx);
        }
        cx.notify();
    }

    /// Leaves Settings for the screen it was opened from.
    pub(crate) fn close_settings(&mut self, cx: &mut Context<Self>) {
        self.route = if self.prev_route == Route::Settings {
            Route::Home
        } else {
            self.prev_route
        };
        self.settings_sub = SettingsSub::General;
        self.sub_inputs = SubInputs::default();
        self.settings_input_subs.clear();
        self.settings_dirty = false;
        self.recording_shortcut = None;
        cx.notify();
    }

    pub(crate) fn start_recording_shortcut(
        &mut self,
        key: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.recording_shortcut = Some(key);
        window.focus(&self.recorder_focus, cx);
        cx.notify();
    }

    /// Esc cancels, Backspace clears, any other combo is saved + re-registered.
    pub(crate) fn on_recorder_key(&mut self, ev: &gpui::KeyDownEvent, cx: &mut Context<Self>) {
        let Some(key) = self.recording_shortcut else {
            return;
        };
        let ks = &ev.keystroke;
        match ks.key.as_str() {
            "escape" => {
                self.recording_shortcut = None;
                cx.notify();
                return;
            }
            "backspace" | "delete" => {
                self.recording_shortcut = None;
                self.set_shortcut(key, String::new(), cx);
                return;
            }
            _ => {}
        }
        if let Some(accel) = crate::ui::pages::keystroke_to_accel(ks) {
            self.recording_shortcut = None;
            self.set_shortcut(key, accel, cx);
        }
    }

    fn set_shortcut(&mut self, key: &str, accel: String, cx: &mut Context<Self>) {
        let mut map = serde_json::Map::new();
        map.insert(
            key.to_string(),
            if accel.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(accel)
            },
        );
        self.set_app_flag(serde_json::Value::Object(map), cx);
    }

    pub(crate) fn save_settings_sub(&mut self, cx: &mut Context<Self>) {
        self.settings_dirty = false;
        cx.notify();
        match self.settings_sub {
            SettingsSub::Tun => {
                let device = self
                    .sub_inputs
                    .device
                    .as_ref()
                    .map(|e| e.read(cx).value().to_string());
                let mtu = self
                    .sub_inputs
                    .mtu
                    .as_ref()
                    .and_then(|e| e.read(cx).value().trim().parse::<u64>().ok());
                let mut patch = serde_json::Map::new();
                if let Some(d) = device {
                    patch.insert("device".into(), serde_json::Value::String(d));
                }
                if let Some(m) = mtu {
                    patch.insert("mtu".into(), serde_json::json!(m));
                }
                self.patch_tun(serde_json::Value::Object(patch), cx);
            }
            SettingsSub::SysProxy => {
                let host = self
                    .sub_inputs
                    .host
                    .as_ref()
                    .map(|e| e.read(cx).value().to_string())
                    .unwrap_or_default();
                let bypass: Vec<String> = self
                    .sub_inputs
                    .bypass
                    .as_ref()
                    .map(|e| e.read(cx).value().to_string())
                    .unwrap_or_default()
                    .split([',', '\n'])
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                self.set_app_flag(
                    serde_json::json!({ "sysProxy": { "host": host, "bypass": bypass } }),
                    cx,
                );
            }
            SettingsSub::Advanced => {
                if let Some(n) = self
                    .sub_inputs
                    .interval
                    .as_ref()
                    .and_then(|e| e.read(cx).value().trim().parse::<u64>().ok())
                {
                    self.set_app_flag(serde_json::json!({ "networkDetectionInterval": n }), cx);
                }
            }
            SettingsSub::Dns => {
                let lines = |inp: &Option<Entity<TextareaState>>| -> Vec<String> {
                    inp.as_ref()
                        .map(|e| e.read(cx).value().to_string())
                        .unwrap_or_default()
                        .split(['\n', ','])
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                };
                let mut dns = serde_json::Map::new();
                if let Some(r) = self
                    .sub_inputs
                    .dns_fakeip_range
                    .as_ref()
                    .map(|e| e.read(cx).value().trim().to_string())
                    .filter(|s| !s.is_empty())
                {
                    dns.insert("fake-ip-range".into(), serde_json::json!(r));
                }
                dns.insert(
                    "nameserver".into(),
                    serde_json::json!(lines(&self.sub_inputs.dns_nameserver)),
                );
                dns.insert(
                    "default-nameserver".into(),
                    serde_json::json!(lines(&self.sub_inputs.dns_default_ns)),
                );
                dns.insert(
                    "fake-ip-filter".into(),
                    serde_json::json!(lines(&self.sub_inputs.dns_fakeip_filter)),
                );
                dns.insert(
                    "proxy-server-nameserver".into(),
                    serde_json::json!(lines(&self.sub_inputs.dns_proxy_ns)),
                );
                dns.insert(
                    "direct-nameserver".into(),
                    serde_json::json!(lines(&self.sub_inputs.dns_direct_ns)),
                );
                self.patch_dns(serde_json::Value::Object(dns), cx);
            }
            SettingsSub::Mihomo => {
                let num = |inp: &Option<Entity<InputState>>| -> Option<u64> {
                    inp.as_ref()
                        .and_then(|e| e.read(cx).value().trim().parse::<u64>().ok())
                };
                let lines = |inp: &Option<Entity<TextareaState>>| -> Vec<String> {
                    inp.as_ref()
                        .map(|e| e.read(cx).value().to_string())
                        .unwrap_or_default()
                        .split(['\n', ','])
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                };
                let mut patch = serde_json::Map::new();
                let mut put_port = |key: &str, inp: &Option<Entity<InputState>>| {
                    if let Some(p) = num(inp) {
                        patch.insert(key.into(), serde_json::json!(p));
                    }
                };
                put_port("mixed-port", &self.sub_inputs.mixed_port);
                put_port("socks-port", &self.sub_inputs.socks_port);
                put_port("port", &self.sub_inputs.http_port);
                put_port("redir-port", &self.sub_inputs.redir_port);
                put_port("tproxy-port", &self.sub_inputs.tproxy_port);
                put_port("keep-alive-idle", &self.sub_inputs.keep_alive_idle);
                put_port("keep-alive-interval", &self.sub_inputs.keep_alive_interval);
                if let Some(iface) = self
                    .sub_inputs
                    .interface_name
                    .as_ref()
                    .map(|e| e.read(cx).value().trim().to_string())
                {
                    patch.insert("interface-name".into(), serde_json::json!(iface));
                }
                patch.insert(
                    "skip-auth-prefixes".into(),
                    serde_json::json!(lines(&self.sub_inputs.skip_auth)),
                );
                patch.insert(
                    "lan-allowed-ips".into(),
                    serde_json::json!(lines(&self.sub_inputs.lan_allowed)),
                );
                patch.insert(
                    "lan-disallowed-ips".into(),
                    serde_json::json!(lines(&self.sub_inputs.lan_disallowed)),
                );
                self.patch_core(serde_json::Value::Object(patch), cx);
            }
            SettingsSub::Sniffer => {
                let lines = |inp: &Option<Entity<TextareaState>>| -> Vec<String> {
                    inp.as_ref()
                        .map(|e| e.read(cx).value().to_string())
                        .unwrap_or_default()
                        .split(['\n', ','])
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                };
                let mut sniffer = serde_json::Map::new();
                sniffer.insert(
                    "force-domain".into(),
                    serde_json::json!(lines(&self.sub_inputs.sniff_force_domain)),
                );
                sniffer.insert(
                    "skip-domain".into(),
                    serde_json::json!(lines(&self.sub_inputs.sniff_skip_domain)),
                );
                sniffer.insert(
                    "skip-dst-address".into(),
                    serde_json::json!(lines(&self.sub_inputs.sniff_skip_dst)),
                );
                sniffer.insert(
                    "skip-src-address".into(),
                    serde_json::json!(lines(&self.sub_inputs.sniff_skip_src)),
                );
                self.patch_sniffer(serde_json::Value::Object(sniffer), cx);
            }
            _ => return,
        }
        cx.defer(|cx| {
            crate::app::actions::notify(ToastKind::Success, t!("common.saved").to_string(), cx);
        });
    }

    pub(crate) fn patch_sniffer(&mut self, patch: serde_json::Value, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let body = serde_json::json!({ "sniffer": patch });
            let _ = runtime::spawn(backend::config::patch_controled_mihomo_config(body)).await;
            let _ = runtime::spawn(backend::core::restart()).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    pub(crate) fn refresh_providers(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let proxies = runtime::spawn(backend::api::get_proxy_providers()).await;
            let rules = runtime::spawn(backend::api::get_rule_providers()).await;
            let _ = this.update(cx, |this, cx| {
                this.proxy_providers = match proxies {
                    Ok(Ok(v)) => parse_providers(&v, false),
                    _ => Vec::new(),
                };
                this.rule_providers = match rules {
                    Ok(Ok(v)) => parse_providers(&v, true),
                    _ => Vec::new(),
                };
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn update_provider(&mut self, name: String, is_rule: bool, cx: &mut Context<Self>) {
        if self.resources_busy {
            return;
        }
        self.resources_busy = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let _ = runtime::spawn(async move {
                if is_rule {
                    backend::api::update_rule_provider(&name).await
                } else {
                    backend::api::update_proxy_provider(&name).await
                }
            })
            .await;
            let _ = this.update(cx, |this, cx| {
                this.resources_busy = false;
                this.refresh_providers(cx);
            });
        })
        .detach();
    }

    pub(crate) fn update_all_providers(&mut self, is_rule: bool, cx: &mut Context<Self>) {
        if self.resources_busy {
            return;
        }
        let names: Vec<String> = if is_rule {
            self.rule_providers
                .iter()
                .map(|p| p.name.to_string())
                .collect()
        } else {
            self.proxy_providers
                .iter()
                .map(|p| p.name.to_string())
                .collect()
        };
        if names.is_empty() {
            return;
        }
        self.resources_busy = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            for name in names {
                let _ = runtime::spawn(async move {
                    if is_rule {
                        backend::api::update_rule_provider(&name).await
                    } else {
                        backend::api::update_proxy_provider(&name).await
                    }
                })
                .await;
            }
            let _ = this.update(cx, |this, cx| {
                this.resources_busy = false;
                this.refresh_providers(cx);
            });
        })
        .detach();
    }

    pub(crate) fn open_provider_viewer(
        &mut self,
        name: String,
        is_rule: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let editor = moonglass::code::editor(moonglass::code::Flavor::Yaml, "", true, window, cx);
        let title = SharedString::from(name.clone());
        let view_editor = editor.clone();
        self.open_dialog(
            760.,
            true,
            move |_, _, cx| {
                let c = cx.theme().colors;
                nyx_dialog(
                    title.clone(),
                    div()
                        .h(px(460.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(c.border)
                        .overflow_hidden()
                        .child(moonglass::code::CodeEditor::new(&view_editor)),
                    None,
                    cx,
                )
            },
            cx,
        );
        cx.spawn_in(window, async move |_this, cx| {
            let content =
                match runtime::spawn(backend::config::read_provider_content(name, is_rule)).await {
                    Ok(Ok(c)) => c,
                    Ok(Err(e)) => format!("# {e}"),
                    Err(_) => "# load cancelled".to_string(),
                };
            let _ = cx.update(|window, cx| {
                editor.update(cx, |st, c| st.set_value(content, window, c));
            });
        })
        .detach();
    }

    pub(crate) fn update_geo(&mut self, cx: &mut Context<Self>) {
        if self.resources_busy {
            return;
        }
        self.resources_busy = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let res = runtime::spawn(backend::api::upgrade_geo()).await;
            if let Ok(Err(e)) = &res {
                log::warn!("[geo] update failed: {e}");
            }
            let ok = matches!(res, Ok(Ok(())));
            cx.update(|cx| {
                let (kind, text) = if ok {
                    (ToastKind::Success, t!("pages.settings.resOk"))
                } else {
                    (ToastKind::Error, t!("pages.settings.resFail"))
                };
                crate::app::actions::notify(kind, text.to_string(), cx);
            });
            let _ = this.update(cx, |this, cx| {
                this.resources_busy = false;
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn patch_core(&mut self, patch: serde_json::Value, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let _ = runtime::spawn(backend::config::patch_controled_mihomo_config(patch)).await;
            let _ = runtime::spawn(backend::core::restart()).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    pub(crate) fn refresh_service_info(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let status = runtime::spawn(backend::core::service_status()).await;
            let version = runtime::spawn(backend::manager::get_installed_version()).await;
            let _ = this.update(cx, |this, cx| {
                this.service_managed = backend::core::service_managed();
                if let Ok(s) = status {
                    this.service_status = s.as_str().into();
                    this.service_detail = match s {
                        backend::core::ServiceStatus::Stale { reason } => Some(reason.into()),
                        _ => None,
                    };
                }
                if let Ok(Ok(v)) = version {
                    this.core_version_installed = v.into();
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn service_action(&mut self, action: &'static str, cx: &mut Context<Self>) {
        if self.service_busy {
            return;
        }
        self.service_busy = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let res = runtime::spawn(async move {
                match action {
                    "install" => backend::core::install_service().await,
                    "uninstall" => backend::core::uninstall_service().await,
                    "start" => backend::core::start_service().await,
                    "stop" => backend::core::stop_service().await,
                    "restart" => backend::core::restart_service().await,
                    _ => Ok(()),
                }
            })
            .await;
            if let Ok(Err(e)) = &res {
                log::warn!("[service] {action} failed: {e}");
                let detail = e.detail.clone();
                cx.update(|cx| crate::app::actions::notify(ToastKind::Error, detail, cx));
            }
            match action {
                "stop" | "uninstall" if matches!(res, Ok(Ok(()))) => {
                    // The core went down with the service — drop the stale state.
                    crate::app::actions::mark_disconnected(cx).await;
                    crate::app::bootstrap::refresh_runtime_data(cx).await;
                }
                // Install brings the core up with TUN off; start/restart restores it.
                "install" if matches!(res, Ok(Ok(()))) => {
                    crate::app::bootstrap::start_core_and_streams(cx, false).await;
                }
                "start" | "restart" if matches!(res, Ok(Ok(()))) => {
                    let tun = crate::app::bootstrap::restore_tun();
                    crate::app::bootstrap::start_core_and_streams(cx, tun).await;
                }
                _ => crate::app::bootstrap::refresh_runtime_data(cx).await,
            }
            let _ = this.update(cx, |this, cx| {
                this.service_busy = false;
                this.refresh_service_info(cx);
            });
        })
        .detach();
    }

    pub(crate) fn install_core(&mut self, channel: &'static str, cx: &mut Context<Self>) {
        if self.service_busy {
            return;
        }
        self.service_busy = true;
        cx.notify();
        self.set_app_flag(serde_json::json!({ "core": channel }), cx);
        cx.spawn(async move |this, cx| {
            let res = runtime::spawn(backend::manager::install_core_for_core_type(channel)).await;
            if let Ok(Err(e)) = &res {
                log::warn!("[core] install {channel} failed: {e}");
                let msg = format!("{e}");
                cx.update(|cx| crate::app::actions::notify(ToastKind::Error, msg, cx));
            }
            let _ = runtime::spawn(backend::core::restart()).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
            let _ = this.update(cx, |this, cx| {
                this.service_busy = false;
                this.refresh_service_info(cx);
            });
        })
        .detach();
    }

    pub(crate) fn patch_tun(&mut self, patch: serde_json::Value, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let body = serde_json::json!({ "tun": patch });
            let _ = runtime::spawn(backend::config::patch_controled_mihomo_config(body)).await;
            let _ = runtime::spawn(backend::core::restart()).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    pub(crate) fn toggle_override(
        &mut self,
        key: &'static str,
        checked: bool,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |_this, cx| {
            let patch = serde_json::json!({ key: checked });
            let _ = runtime::spawn(backend::config::patch_app_config(patch)).await;
            let _ = runtime::spawn(backend::core::restart()).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    pub(crate) fn patch_dns(&mut self, patch: serde_json::Value, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let body = serde_json::json!({ "dns": patch });
            let _ = runtime::spawn(backend::config::patch_controled_mihomo_config(body)).await;
            let _ = runtime::spawn(backend::core::restart()).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    pub(crate) fn close_connections(&mut self, ids: Vec<String>, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, _cx| {
            for id in ids {
                let _ =
                    runtime::spawn(async move { backend::api::close_connection(&id).await }).await;
            }
        })
        .detach();
    }

    pub(crate) fn close_all_connections(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, _cx| {
            let _ = runtime::spawn(backend::api::close_all_connections()).await;
        })
        .detach();
    }
}
