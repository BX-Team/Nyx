use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, rgb, rgba, size, App, AppContext, Context, Entity, InteractiveElement, IntoElement,
    ParentElement, PathPromptOptions, Render, ScrollHandle, StatefulInteractiveElement, Styled,
    Subscription, Window, WindowBounds, WindowOptions,
};
use gpui_component::input::{Input, InputState};
use gpui_component::select::{SelectEvent, SelectState};
use gpui_component::IndexPath;
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex,
    text::TextView,
    v_flex, Disableable, Root, StyledExt, TitleBar,
};
use rust_i18n::t;

use crate::app::runtime;
use crate::app::state::{parse_groups, AppState};
use crate::backend;

// Nyx palette + gradients live in `ui::theme`; re-export so pages keep
// importing color tokens from `crate::ui::root::*`.
pub(crate) use crate::ui::theme::*;

/// Top-level navigation targets.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Route {
    Home,
    Profiles,
    Proxies,
    Rules,
    Connections,
    Logs,
    Settings,
}

/// Log-level filter for the Logs page segmented control.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogFilter {
    All,
    Info,
    Warning,
    Error,
}

/// Settings detail sub-pages opened from the gear icons / section rows.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingsSub {
    Tun,
    SysProxy,
    Dns,
    Mihomo,
    Sniffer,
    Resources,
    Appearance,
    Advanced,
    Shortcuts,
}

/// A rule/proxy provider row on the Resources page.
#[derive(Clone)]
pub(crate) struct ProviderRow {
    pub(crate) name: gpui::SharedString,
    pub(crate) subtitle: gpui::SharedString,
}

/// The Resources page's provider-content viewer (a read-only code editor modal).
pub(crate) struct ProviderViewerState {
    pub(crate) title: String,
    pub(crate) editor: Entity<InputState>,
}

/// Text inputs owned by the active Settings sub-page (created on open).
#[derive(Default)]
pub(crate) struct SubInputs {
    pub(crate) device: Option<Entity<InputState>>,
    pub(crate) mtu: Option<Entity<InputState>>,
    pub(crate) host: Option<Entity<InputState>>,
    pub(crate) bypass: Option<Entity<InputState>>,
    pub(crate) interval: Option<Entity<InputState>>,
    pub(crate) dns_fakeip_range: Option<Entity<InputState>>,
    pub(crate) dns_nameserver: Option<Entity<InputState>>,
    pub(crate) dns_default_ns: Option<Entity<InputState>>,
    pub(crate) dns_fakeip_filter: Option<Entity<InputState>>,
    pub(crate) dns_proxy_ns: Option<Entity<InputState>>,
    pub(crate) dns_direct_ns: Option<Entity<InputState>>,
    pub(crate) mixed_port: Option<Entity<InputState>>,
    pub(crate) socks_port: Option<Entity<InputState>>,
    pub(crate) http_port: Option<Entity<InputState>>,
    pub(crate) redir_port: Option<Entity<InputState>>,
    pub(crate) tproxy_port: Option<Entity<InputState>>,
    pub(crate) keep_alive_idle: Option<Entity<InputState>>,
    pub(crate) keep_alive_interval: Option<Entity<InputState>>,
    pub(crate) interface_name: Option<Entity<InputState>>,
    pub(crate) skip_auth: Option<Entity<InputState>>,
    pub(crate) lan_allowed: Option<Entity<InputState>>,
    pub(crate) lan_disallowed: Option<Entity<InputState>>,
    pub(crate) sniff_force_domain: Option<Entity<InputState>>,
    pub(crate) sniff_skip_domain: Option<Entity<InputState>>,
    pub(crate) sniff_skip_dst: Option<Entity<InputState>>,
    pub(crate) sniff_skip_src: Option<Entity<InputState>>,
}

/// What the embedded YAML editor is currently editing.
#[derive(Clone)]
pub(crate) enum EditorTarget {
    Profile { id: String, name: String },
    RuntimeReadonly,
}

/// State of the smart rule-override editor: `prepend`/`append` custom rules plus
/// the subscription's read-only rules.
pub(crate) struct RuleEditState {
    pub(crate) profile_id: String,
    pub(crate) profile_name: String,
    pub(crate) prepend: Vec<String>,
    pub(crate) append: Vec<String>,
    /// Subscription rule strings the user has chosen to drop (override `delete`).
    pub(crate) delete: Vec<String>,
    /// "Add rule" form: type, payload, target policy (a dropdown of live groups/
    /// nodes + DIRECT/REJECT/…), and where to insert.
    pub(crate) type_select: Entity<SelectState<Vec<gpui::SharedString>>>,
    pub(crate) payload: Entity<InputState>,
    pub(crate) policy_select: Entity<SelectState<Vec<gpui::SharedString>>>,
    pub(crate) to_append: bool,
    /// Keeps the payload placeholder in sync with the picked rule type.
    _type_sub: Subscription,
}

/// Root view: custom title bar + sidebar + routed content.
pub(crate) struct NyxApp {
    pub(crate) state: Entity<AppState>,
    pub(crate) route: Route,
    /// First-run welcome flow: `Some(step)` while active (0..=3), `None` once done.
    pub(crate) onboarding_step: Option<u8>,
    pub(crate) rail_expanded: bool,
    /// Currently focused proxy group on the Proxies page (right-hand node grid).
    pub(crate) proxies_group: Option<gpui::SharedString>,
    /// Proxies page node-grid controls: search, sort-by-latency, alive-only.
    pub(crate) proxies_search: Entity<InputState>,
    pub(crate) proxies_sort_latency: bool,
    pub(crate) proxies_alive_only: bool,
    pub(crate) logs_filter: LogFilter,
    /// Connections page: process-name filter + the process whose detail is open.
    pub(crate) conns_filter: Entity<InputState>,
    pub(crate) conns_detail: Option<gpui::SharedString>,
    /// Connections page tab: `false` = active, `true` = recently closed.
    pub(crate) conns_show_closed: bool,
    /// A single connection selected for the detail popup (within a process).
    pub(crate) conn_detail_item: Option<crate::app::state::ConnItem>,
    /// Scroll handle for the Logs console (used to stick to the bottom).
    pub(crate) logs_scroll: ScrollHandle,
    /// Total log count last rendered — autoscroll fires when it grows.
    pub(crate) logs_seen: std::cell::Cell<usize>,
    /// Active Settings sub-page, if any (gear / section navigation).
    pub(crate) settings_sub: Option<SettingsSub>,
    pub(crate) sub_inputs: SubInputs,
    /// Shortcuts page: the app-config key currently being recorded, if any.
    pub(crate) recording_shortcut: Option<&'static str>,
    pub(crate) recorder_focus: gpui::FocusHandle,
    /// Mihomo settings: Windows service status + installed core version, plus a busy guard.
    pub(crate) service_status: gpui::SharedString,
    pub(crate) core_version_installed: gpui::SharedString,
    pub(crate) service_busy: bool,
    /// Resources page: fetched providers + an in-flight guard for geo/provider updates.
    pub(crate) proxy_providers: Vec<ProviderRow>,
    pub(crate) rule_providers: Vec<ProviderRow>,
    pub(crate) resources_busy: bool,
    /// Open provider-content viewer modal (Resources page), if any.
    pub(crate) provider_viewer: Option<ProviderViewerState>,
    pub(crate) editor: Option<Entity<InputState>>,
    pub(crate) editor_target: Option<EditorTarget>,
    /// Active smart rule editor, if open (Rules page).
    pub(crate) rule_editor: Option<RuleEditState>,
    pub(crate) import_url: Entity<InputState>,
    /// "Add profile" modal: open flag, remote/local toggle, name, picked local file.
    pub(crate) profile_add_open: bool,
    pub(crate) profile_add_local: bool,
    pub(crate) profile_add_name: Entity<InputState>,
    /// Auto-update interval input (hours, remote profiles); empty = off.
    pub(crate) profile_interval: Entity<InputState>,
    pub(crate) profile_add_file: Option<(String, String)>,
    /// Id of the profile being edited; `None` when the modal is creating a new one.
    pub(crate) profile_edit_id: Option<String>,
    /// Set while a profile import is downloading; keeps the modal open + disabled.
    pub(crate) profile_add_busy: bool,
    /// Last import error, shown inline in the modal.
    pub(crate) profile_add_error: Option<gpui::SharedString>,
    /// MRS converter modal: open flag, input file, mihomo behavior.
    pub(crate) mrs_open: bool,
    pub(crate) mrs_input: Option<std::path::PathBuf>,
    pub(crate) mrs_behavior: &'static str,
    pub(crate) connected_since: Option<std::time::Instant>,
    pub(crate) stats_open: bool,
    /// Settings language picker (a real dropdown over [`LANGUAGES`]).
    pub(crate) lang_select: Entity<SelectState<Vec<gpui::SharedString>>>,
    /// Auto-updater: pending newer release + in-flight flags + modal open.
    pub(crate) update_info: Option<backend::updater::UpdateInfo>,
    pub(crate) update_checking: bool,
    pub(crate) update_installing: bool,
    pub(crate) updater_open: bool,
    /// Whether the "reset application" confirmation dialog is open.
    pub(crate) reset_confirm_open: bool,
    /// Guards the one-time silent auto-check after config loads.
    auto_update_checked: bool,
    _state_sub: Subscription,
    _lang_sub: Subscription,
    _conns_filter_sub: Subscription,
    _proxies_search_sub: Subscription,
}

/// Human-readable byte count (e.g. `1.2 MB`).
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

/// Human-readable transfer rate (e.g. `1.2 MB/s`).
pub(crate) fn fmt_speed(n: u64) -> String {
    format!("{}/s", fmt_bytes(n))
}

/// Parses a `/providers/{proxies,rules}` response into displayable rows,
/// skipping built-in `Compatible` providers (which can't be updated).
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

/// Latency color: green (fast), amber (ok), red (slow), muted (untested).
pub(crate) fn delay_color(delay: Option<u32>) -> u32 {
    match delay {
        None => MUTED,
        Some(d) if d < 200 => GOOD,
        Some(d) if d < 500 => WARN,
        Some(_) => BAD,
    }
}

impl NyxApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = AppState::global(cx);
        let import_url = cx.new(|cx| InputState::new(window, cx).placeholder("https://…"));
        let profile_add_name = cx.new(|cx| InputState::new(window, cx).placeholder("Name"));
        let profile_interval = cx.new(|cx| InputState::new(window, cx).placeholder("0"));
        let conns_filter = cx.new(|cx| InputState::new(window, cx));
        // Re-render the connections list as the user types in the filter box.
        let conns_filter_sub = cx.subscribe(
            &conns_filter,
            |_this, _input, _event: &gpui_component::input::InputEvent, cx| cx.notify(),
        );
        let proxies_search = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("pages.proxies.searchPlaceholder").to_string())
        });
        let proxies_search_sub = cx.subscribe(
            &proxies_search,
            |_this, _input, _event: &gpui_component::input::InputEvent, cx| cx.notify(),
        );
        // Re-render on shared-state change; track TUN up-time for Home's timer.
        let sub = cx.observe(&state, |this: &mut Self, observed, cx| {
            let connected = observed.read(cx).tun_enabled;
            if connected && this.connected_since.is_none() {
                this.connected_since = Some(std::time::Instant::now());
            } else if !connected {
                this.connected_since = None;
            }
            // One-time silent update check once config loads, if auto-check is on.
            if !this.auto_update_checked && !observed.read(cx).app_config.is_null() {
                this.auto_update_checked = true;
                if observed.read(cx).app_flag("autoCheckUpdate") {
                    this.check_update(true, cx);
                }
            }
            cx.notify();
        });

        // Language dropdown over LANGUAGES, preselected to the active locale.
        use crate::app::state::LANGUAGES;
        let current_lang = state.read(cx).language.clone();
        let names: Vec<gpui::SharedString> =
            LANGUAGES.iter().map(|(_, name)| (*name).into()).collect();
        let cur_idx = LANGUAGES
            .iter()
            .position(|(code, _)| *code == current_lang.as_ref())
            .unwrap_or(0);
        let lang_select = cx
            .new(|cx| SelectState::new(names, Some(IndexPath::default().row(cur_idx)), window, cx));
        let lang_sub = cx.subscribe(&lang_select, Self::on_language_selected);

        let onboarding_pending = !backend::config::app_config_bool("onboardingDone");
        state.update(cx, |s, _| s.onboarding_active = onboarding_pending);

        Self {
            state,
            route: Route::Home,
            onboarding_step: if onboarding_pending { Some(0) } else { None },
            rail_expanded: false,
            proxies_group: None,
            proxies_search,
            proxies_sort_latency: false,
            proxies_alive_only: false,
            logs_filter: crate::ui::root::LogFilter::All,
            conns_filter,
            conns_detail: None,
            conns_show_closed: false,
            conn_detail_item: None,
            logs_scroll: ScrollHandle::new(),
            logs_seen: std::cell::Cell::new(0),
            settings_sub: None,
            sub_inputs: SubInputs::default(),
            recording_shortcut: None,
            recorder_focus: cx.focus_handle(),
            service_status: gpui::SharedString::default(),
            core_version_installed: gpui::SharedString::default(),
            service_busy: false,
            proxy_providers: Vec::new(),
            rule_providers: Vec::new(),
            resources_busy: false,
            provider_viewer: None,
            editor: None,
            editor_target: None,
            rule_editor: None,
            import_url,
            profile_add_open: false,
            profile_add_local: false,
            profile_add_name,
            profile_interval,
            profile_add_file: None,
            profile_edit_id: None,
            profile_add_busy: false,
            profile_add_error: None,
            mrs_open: false,
            mrs_input: None,
            mrs_behavior: "domain",
            connected_since: None,
            stats_open: true,
            lang_select,
            update_info: None,
            update_checking: false,
            update_installing: false,
            updater_open: false,
            reset_confirm_open: false,
            auto_update_checked: false,
            _state_sub: sub,
            _lang_sub: lang_sub,
            _conns_filter_sub: conns_filter_sub,
            _proxies_search_sub: proxies_search_sub,
        }
    }

    /// Checks GitHub for a newer release and opens the updater modal if one
    /// exists. When not `silent`, also toasts the outcome.
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
                        this.updater_open = true;
                    }
                    Ok(None) if !silent => crate::app::actions::notify(
                        gpui_component::notification::Notification::info(t!("updater.upToDate")),
                        cx,
                    ),
                    Ok(None) => {}
                    Err(e) if !silent => crate::app::actions::notify(
                        gpui_component::notification::Notification::error(format!(
                            "{}: {e}",
                            t!("updater.checkFailed")
                        )),
                        cx,
                    ),
                    Err(e) => log::warn!("[updater] auto-check failed: {e}"),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Downloads + installs the pending update, then relaunches.
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
                Ok(()) => {
                    cx.update(crate::app::actions::restart_app);
                }
                Err(e) => {
                    let _ = this.update(cx, |this, cx| {
                        this.update_installing = false;
                        crate::app::actions::notify(
                            gpui_component::notification::Notification::error(format!(
                                "{}: {e}",
                                t!("updater.installFailed")
                            )),
                            cx,
                        );
                        cx.notify();
                    });
                }
            }
        })
        .detach();
    }

    /// Closes the updater modal (keeps the pending info for a later open).
    pub(crate) fn close_updater(&mut self, cx: &mut Context<Self>) {
        self.updater_open = false;
        cx.notify();
    }

    /// Opens the "reset application" confirmation dialog.
    pub(crate) fn open_reset_confirm(&mut self, cx: &mut Context<Self>) {
        self.reset_confirm_open = true;
        cx.notify();
    }

    /// Dismisses the reset confirmation dialog without resetting.
    pub(crate) fn close_reset_confirm(&mut self, cx: &mut Context<Self>) {
        self.reset_confirm_open = false;
        cx.notify();
    }

    /// Wipes all app data then relaunches the app (confirmed reset).
    pub(crate) fn confirm_reset(&mut self, cx: &mut Context<Self>) {
        self.reset_confirm_open = false;
        cx.notify();
        cx.spawn(async move |_this, cx| {
            let _ = runtime::spawn(backend::config::reset_app_config()).await;
            cx.update(crate::app::actions::restart_app);
        })
        .detach();
    }

    /// Applies a language picked in the Settings dropdown.
    fn on_language_selected(
        &mut self,
        _select: Entity<SelectState<Vec<gpui::SharedString>>>,
        event: &SelectEvent<Vec<gpui::SharedString>>,
        cx: &mut Context<Self>,
    ) {
        use crate::app::state::LANGUAGES;
        let SelectEvent::Confirm(Some(name)) = event else {
            return;
        };
        if let Some((code, _)) = LANGUAGES.iter().find(|(_, n)| *n == name.as_ref()) {
            self.state.update(cx, |s, c| s.set_language(*code, c));
            crate::app::tray::rebuild(cx);
        }
    }
}

impl NyxApp {
    pub(crate) fn toggle_tun(&mut self, cx: &mut Context<Self>) {
        let new = !self.state.read(cx).tun_enabled;
        let running = self.state.read(cx).core_status.is_running();
        self.state.update(cx, |st, c| st.set_tun_enabled(new, c));
        cx.spawn(async move |_this, cx| {
            if !running && !crate::app::bootstrap::start_core_and_streams(cx).await {
                cx.update(|cx| {
                    AppState::global(cx).update(cx, |st, c| st.set_tun_enabled(false, c));
                });
                return;
            }
            let patch = if new {
                serde_json::json!({ "tun": { "enable": true }, "dns": { "enable": true } })
            } else {
                serde_json::json!({ "tun": { "enable": false } })
            };
            let _ = runtime::spawn(backend::config::patch_controled_mihomo_config(patch)).await;
            let _ = runtime::spawn(backend::config::patch_app_config(
                serde_json::json!({ "lastConnected": new }),
            ))
            .await;
            if let Ok(Ok(cfg)) =
                runtime::spawn(backend::config::get_controled_mihomo_config()).await
            {
                let tun = cfg
                    .get("tun")
                    .and_then(|t| t.get("enable"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(new);
                cx.update(|cx| {
                    AppState::global(cx).update(cx, |st, c| st.set_tun_enabled(tun, c));
                });
            }
        })
        .detach();
    }

    /// Selects `proxy` within `group`, then refreshes the group list.
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

    /// Latency-tests a single proxy and stores the result on its node.
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

    /// Latency-tests an entire group, updating every member's delay.
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

    /// Manually re-fetches the proxy group list.
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
            let note = if ok {
                gpui_component::notification::Notification::success(t!("pages.home.subUpdated"))
            } else {
                gpui_component::notification::Notification::error(t!("pages.home.subUpdateFailed"))
            };
            cx.update(|cx| crate::app::actions::notify(note, cx));
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

    /// Closes and re-establishes all active connections through the current rules.
    pub(crate) fn restart_connections(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, _cx| {
            let _ = runtime::spawn(backend::api::restart_connections()).await;
        })
        .detach();
    }

    pub(crate) fn open_mrs_convert(&mut self, cx: &mut Context<Self>) {
        self.mrs_open = true;
        self.mrs_input = None;
        cx.notify();
    }

    pub(crate) fn close_mrs_convert(&mut self, cx: &mut Context<Self>) {
        self.mrs_open = false;
        cx.notify();
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
        self.mrs_open = false;
        cx.notify();
        cx.spawn(async move |_this, cx| {
            let p = input.to_string_lossy().to_string();
            let res = runtime::spawn(backend::config::convert_mrs_ruleset(
                p,
                behavior.to_string(),
            ))
            .await;
            let note = match res {
                Ok(Ok(content)) => {
                    let stem = input
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "ruleset".to_string());
                    let out = input.with_file_name(format!("{stem}-{behavior}.txt"));
                    match std::fs::write(&out, content) {
                        Ok(_) => gpui_component::notification::Notification::info(format!(
                            "{}: {}",
                            t!("pages.rules.convertDone"),
                            out.display()
                        )),
                        Err(e) => gpui_component::notification::Notification::error(e.to_string()),
                    }
                }
                Ok(Err(e)) => gpui_component::notification::Notification::error(e),
                Err(_) => gpui_component::notification::Notification::error(
                    t!("pages.rules.convertFailed").to_string(),
                ),
            };
            cx.update(|cx| crate::app::actions::notify(note, cx));
        })
        .detach();
    }
}

/// Fetches groups on the tokio runtime and folds them into `AppState`.
async fn refresh_groups(cx: &mut gpui::AsyncApp) {
    if let Ok(Ok(val)) = runtime::spawn(backend::mihomo::groups()).await {
        cx.update(|cx| {
            let parsed = parse_groups(&val);
            AppState::global(cx).update(cx, |st, c| st.set_groups(parsed, c));
        });
    }
}

impl NyxApp {
    fn render_content(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_profiles = !self.state.read(cx).profiles.is_empty();
        let route = if !has_profiles
            && !matches!(self.route, Route::Home | Route::Profiles | Route::Settings)
        {
            Route::Home
        } else {
            self.route
        };
        let body = if self.rule_editor.is_some() {
            self.render_rule_editor(window, cx).into_any_element()
        } else if self.editor_target.is_some() {
            self.render_editor(cx).into_any_element()
        } else {
            match route {
                Route::Home => self.render_home(window, cx).into_any_element(),
                Route::Proxies => self.render_proxies(window, cx).into_any_element(),
                Route::Profiles => self.render_profiles(window, cx).into_any_element(),
                Route::Rules => self.render_rules(cx).into_any_element(),
                Route::Connections => self.render_connections(cx).into_any_element(),
                Route::Logs => self.render_logs(cx).into_any_element(),
                Route::Settings => self.render_settings(cx).into_any_element(),
            }
        };
        v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .text_color(rgb(TEXT))
            .child(body)
    }

    #[allow(dead_code)]
    fn render_placeholder(&self, route: Route) -> impl IntoElement {
        let title = match route {
            Route::Home => t!("sider.home"),
            Route::Profiles => t!("sider.profileManagement"),
            Route::Proxies => t!("sider.proxyGroup"),
            Route::Rules => t!("sider.rules"),
            Route::Connections => t!("sider.connection"),
            Route::Logs => t!("sider.logs"),
            Route::Settings => t!("common.settings"),
        };
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_2()
            .child(div().text_xl().child(title.to_string()))
            .child(
                div()
                    .text_color(rgb(MUTED))
                    .child(t!("page.placeholder").to_string()),
            )
    }
}

impl NyxApp {
    /// The auto-updater modal (version, changelog, Later/Update). Rendered while `updater_open`.
    fn render_updater_modal(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (version, changelog) = self
            .update_info
            .clone()
            .map(|i| (i.version, i.changelog))
            .unwrap_or_default();
        let installing = self.update_installing;
        let install_label = if installing {
            t!("updater.installing")
        } else {
            t!("updater.update")
        };

        div()
            .id("updater-scrim")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x000000B0))
            .child(
                v_flex()
                    .w(px(440.))
                    .max_h(px(520.))
                    .rounded_xl()
                    .border_1()
                    .border_color(rgba(STROKE))
                    .bg(rgb(CARD_BG))
                    .p_5()
                    .gap_3()
                    .child(
                        div()
                            .text_lg()
                            .font_bold()
                            .text_color(rgb(TEXT))
                            .child(t!("updater.versionReady", version => version).to_string()),
                    )
                    .child(
                        div()
                            .id("updater-changelog")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(TextView::markdown("updater-changelog-md", changelog)),
                    )
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("updater-later")
                                    .ghost()
                                    .label(t!("updater.later").to_string())
                                    .disabled(installing)
                                    .on_click(cx.listener(|this, _, _, cx| this.close_updater(cx))),
                            )
                            .child(
                                Button::new("updater-install")
                                    .primary()
                                    .label(install_label.to_string())
                                    .disabled(installing)
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.install_update(cx)),
                                    ),
                            ),
                    ),
            )
    }
}

impl NyxApp {
    /// The "reset application" confirmation dialog. Rendered while `reset_confirm_open`.
    fn render_reset_confirm(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("reset-scrim")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x000000B0))
            .occlude()
            .on_click(cx.listener(|this, _, _, cx| this.close_reset_confirm(cx)))
            .child(
                v_flex()
                    .w(px(420.))
                    .rounded_xl()
                    .border_1()
                    .border_color(rgba(STROKE))
                    .bg(rgb(CARD_BG))
                    .p_5()
                    .gap_3()
                    .id("reset-card")
                    .occlude()
                    .child(
                        div()
                            .text_lg()
                            .font_bold()
                            .text_color(rgb(TEXT))
                            .child(t!("pages.settings.confirmReset").to_string()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(t!("pages.settings.resetWarning").to_string()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(RED_HI))
                            .child(t!("pages.settings.cannotUndo").to_string()),
                    )
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("reset-cancel")
                                    .ghost()
                                    .label(t!("common.cancel").to_string())
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.close_reset_confirm(cx)),
                                    ),
                            )
                            .child(
                                Button::new("reset-confirm")
                                    .danger()
                                    .label(t!("pages.settings.reset").to_string())
                                    .on_click(cx.listener(|this, _, _, cx| this.confirm_reset(cx))),
                            ),
                    ),
            )
    }
}

impl NyxApp {
    /// The Resources provider-content viewer: a read-only editor of the picked provider.
    fn render_provider_viewer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(viewer) = self.provider_viewer.as_ref() else {
            return div().into_any_element();
        };
        let title = viewer.title.clone();
        let editor = viewer.editor.clone();
        div()
            .id("provider-viewer-scrim")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x000000B0))
            .occlude()
            .on_click(cx.listener(|this, _, _, cx| this.close_provider_viewer(cx)))
            .child(
                v_flex()
                    .w(px(720.))
                    .h(px(560.))
                    .max_w(px(900.))
                    .rounded_xl()
                    .border_1()
                    .border_color(rgba(STROKE))
                    .bg(rgb(CARD_BG))
                    .p_4()
                    .gap_3()
                    .id("provider-viewer-card")
                    .occlude()
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_base()
                                    .font_bold()
                                    .text_color(rgb(TEXT))
                                    .truncate()
                                    .child(title),
                            )
                            .child(
                                Button::new("provider-viewer-close")
                                    .ghost()
                                    .label(t!("common.close").to_string())
                                    .on_click(
                                        cx.listener(|this, _, _, cx| {
                                            this.close_provider_viewer(cx)
                                        }),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .border_1()
                            .border_color(rgb(CARD_BORDER))
                            .rounded_lg()
                            .child(Input::new(&editor).h_full().w_full().disabled(true)),
                    ),
            )
            .into_any_element()
    }
}

impl Render for NyxApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The top-level view must render `Root`'s overlay layers itself, or
        // toasts/modals never appear.
        let dialog_layer = Root::render_dialog_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);
        let updater_modal = self.updater_open.then(|| self.render_updater_modal(cx));
        let reset_modal = self
            .reset_confirm_open
            .then(|| self.render_reset_confirm(cx));
        let provider_viewer_modal = self
            .provider_viewer
            .is_some()
            .then(|| self.render_provider_viewer(cx));
        let profile_add_modal = self
            .profile_add_open
            .then(|| self.render_profile_add_modal(cx));
        let mrs_modal = self.mrs_open.then(|| self.render_mrs_modal(cx));
        let onboarding = self.onboarding_active().then(|| self.render_onboarding(cx));

        v_flex()
            .size_full()
            .bg(rgb(TITLEBAR_BG))
            .child(
                TitleBar::new().child(
                    h_flex()
                        .w_full()
                        .pl_2()
                        .items_center()
                        .text_color(rgb(SUBTLE))
                        .text_size(px(12.5))
                        .font_semibold()
                        .child("Nyx"),
                ),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .bg(content_bg())
                    .child(self.render_rail(cx))
                    .child(self.render_content(window, cx)),
            )
            // Onboarding card sits below the modals so dialogs open above it.
            .children(onboarding)
            .children(updater_modal)
            .children(reset_modal)
            .children(provider_viewer_modal)
            .children(profile_add_modal)
            .children(mrs_modal)
            .children(dialog_layer)
            .children(notification_layer)
    }
}

/// Reads `window.window_bounds()` and persists the restore geometry into the app
/// config. Called from the close/hide path (no live gpui borrow conflict).
fn save_main_window_bounds(window: &Window) {
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

/// Opens the main application window. When `silent` is set (silent-start), the
/// window is created but immediately hidden to the tray.
pub fn open_main_window(cx: &mut App, silent: bool) {
    let window_bounds = match backend::config::load_window_state() {
        Some((x, y, w, h)) if w >= 400.0 && h >= 300.0 => WindowBounds::Windowed(gpui::Bounds {
            origin: gpui::point(px(x as f32), px(y as f32)),
            size: size(px(w as f32), px(h as f32)),
        }),
        _ => WindowBounds::centered(size(px(1000.0), px(700.0)), cx),
    };
    cx.spawn(async move |cx| {
        let options = WindowOptions {
            titlebar: Some(TitleBar::title_bar_options()),
            window_bounds: Some(window_bounds),
            window_min_size: Some(size(px(800.0), px(600.0))),
            ..Default::default()
        };

        let handle = cx
            .open_window(options, |window, cx| {
                // Sets the OS window title so the taskbar shows "Nyx" on hover.
                window.set_window_title("Nyx");
                let view = cx.new(|cx| NyxApp::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("failed to open main window");
        cx.update(|cx| {
            crate::app::actions::set_main_window(handle, cx);
            // Close-to-tray: X hides the window; Ctrl+close disconnects the proxy
            // and quits, leaving the core running in the background.
            let _ = handle.update(cx, |_root, window, cx| {
                crate::app::window::remember(window);
                #[cfg(not(windows))]
                if silent {
                    crate::app::window::hide(window);
                }
                window.on_window_should_close(cx, |window, cx| {
                    save_main_window_bounds(window);
                    if window.modifiers().control {
                        crate::app::actions::disconnect_and_quit(cx);
                        return true;
                    }
                    // `spawn` so the Win32 hide runs outside this borrow (else it re-enters).
                    #[cfg(windows)]
                    {
                        let _ = window;
                        cx.spawn(async move |_cx| crate::app::window::hide_now())
                            .detach();
                    }
                    #[cfg(not(windows))]
                    {
                        let _ = cx;
                        crate::app::window::hide(window);
                    }
                    false
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
    /// Re-fetches profiles, groups, tun, version from the backend.
    fn refresh_all(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    /// Opens the "Add profile" modal with a clean form.
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
        self.profile_add_open = true;
        cx.notify();
    }

    /// Opens the modal pre-filled with an existing profile's name/link for editing.
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
        self.profile_add_open = true;
        cx.notify();
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

    /// Closes the "Add profile" modal.
    pub(crate) fn close_profile_add(&mut self, cx: &mut Context<Self>) {
        if self.profile_add_busy {
            return;
        }
        self.profile_add_open = false;
        self.profile_edit_id = None;
        self.profile_add_error = None;
        cx.notify();
    }

    /// Switches the modal between remote (URL) and local (file) sources.
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

    /// Validates + submits the add/edit modal, then refreshes and closes. Edit
    /// mode updates in place (a remote save re-fetches from the URL).
    pub(crate) fn submit_profile_add(&mut self, cx: &mut Context<Self>) {
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
                    cx.spawn(async move |this, cx| {
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
        cx.spawn(async move |this, cx| {
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

    /// Closes the modal on success, or surfaces the error in the still-open modal.
    async fn finish_profile_add(
        this: gpui::WeakEntity<Self>,
        cx: &mut gpui::AsyncApp,
        err: Option<String>,
    ) {
        if err.is_none() {
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        }
        let _ = this.update(cx, |this, cx| {
            this.profile_add_busy = false;
            match err {
                None => {
                    this.profile_add_open = false;
                    this.profile_edit_id = None;
                    this.profile_add_error = None;
                }
                Some(e) => {
                    log::warn!("[profile] import failed: {e}");
                    this.profile_add_error = Some(e.into());
                }
            }
            cx.notify();
        });
    }

    /// Activates a profile and hot-reloads the core.
    pub(crate) fn activate_profile(&mut self, id: String, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let _ = runtime::spawn(backend::config::change_current_profile(id)).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    /// Deletes a profile.
    pub(crate) fn delete_profile(&mut self, id: String, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let _ = runtime::spawn(backend::config::remove_profile_item(id)).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    /// Refreshes a remote profile (re-downloads).
    pub(crate) fn update_profile(&mut self, id: String, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            if let Ok(Ok(item)) = runtime::spawn(backend::config::get_profile_item(id)).await {
                let _ = runtime::spawn(backend::config::add_profile_item(item)).await;
            }
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    /// Re-downloads every remote profile.
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
    /// Opens the YAML editor on a profile's content.
    pub(crate) fn open_profile_editor(
        &mut self,
        id: String,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let editor = cx.new(|cx| InputState::new(window, cx).code_editor("yaml"));
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

    /// Persists the editor content and hot-reloads the core.
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

    /// Closes the editor and returns to the underlying page.
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
    /// Opens the smart rule-override editor on the current profile.
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

        let types: Vec<gpui::SharedString> = crate::ui::pages::RULE_TYPES
            .iter()
            .map(|s| (*s).into())
            .collect();
        let type_select =
            cx.new(|cx| SelectState::new(types, Some(IndexPath::default()), window, cx));
        // Prefill the payload placeholder with the first type's example.
        let first_example = crate::ui::pages::rule_example(crate::ui::pages::RULE_TYPES[0]);
        let payload = cx.new(|cx| InputState::new(window, cx).placeholder(first_example));

        // Policy dropdown: live groups + nodes, then built-in policies; default DIRECT.
        let mut policies: Vec<gpui::SharedString> = Vec::new();
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
        let direct_idx = policies.iter().position(|p| p.as_ref() == "DIRECT");
        let policy_select = cx.new(|cx| {
            SelectState::new(
                policies,
                direct_idx.map(|i| IndexPath::default().row(i)),
                window,
                cx,
            )
        });

        // When the rule type changes, refresh the payload placeholder example.
        let payload_for_sub = payload.clone();
        let type_sub = cx.subscribe_in(
            &type_select,
            window,
            move |_this, _sel, ev: &SelectEvent<Vec<gpui::SharedString>>, window, cx| {
                if let SelectEvent::Confirm(Some(kind)) = ev {
                    let example = crate::ui::pages::rule_example(kind.as_ref());
                    payload_for_sub.update(cx, |s, c| s.set_placeholder(example, window, c));
                }
            },
        );

        self.rule_editor = Some(RuleEditState {
            profile_id: id.clone(),
            profile_name: name,
            prepend: Vec::new(),
            append: Vec::new(),
            delete: Vec::new(),
            type_select,
            payload,
            policy_select,
            to_append: true,
            _type_sub: type_sub,
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

    /// Switches whether a newly added rule goes to the top (prepend) or bottom (append).
    pub(crate) fn rule_editor_set_append(&mut self, to_append: bool, cx: &mut Context<Self>) {
        if let Some(re) = self.rule_editor.as_mut() {
            re.to_append = to_append;
            cx.notify();
        }
    }

    /// Adds the rule described by the form to prepend/append.
    pub(crate) fn rule_editor_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(re) = self.rule_editor.as_ref() else {
            return;
        };
        let kind = re
            .type_select
            .read(cx)
            .selected_value()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "DOMAIN".to_string());
        let payload = re.payload.read(cx).value().trim().to_string();
        let policy = re
            .policy_select
            .read(cx)
            .selected_value()
            .map(|s| s.to_string())
            .unwrap_or_default();
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

    /// Removes a custom (prepend/append) rule by index.
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

    /// Toggles a subscription rule's membership in the `delete` set.
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

    /// Serializes the override file (`prepend`/`append`/`delete`) and reloads.
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

    /// Closes the rule editor without saving.
    pub(crate) fn close_rule_editor(&mut self, cx: &mut Context<Self>) {
        self.rule_editor = None;
        cx.notify();
    }
}

impl NyxApp {
    fn render_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let editor = self.editor.clone();
        let (title, readonly) = match &self.editor_target {
            Some(EditorTarget::Profile { name, .. }) => (name.clone(), false),
            Some(EditorTarget::RuntimeReadonly) => ("Runtime config".to_string(), true),
            None => (String::new(), true),
        };

        let header = h_flex()
            .items_center()
            .justify_between()
            .child(div().text_lg().text_color(rgb(TEXT)).child(title))
            .child(
                h_flex()
                    .gap_2()
                    .when(!readonly, |this| {
                        this.child(
                            Button::new("editor-save")
                                .label(t!("common.save").to_string())
                                .on_click(cx.listener(|this, _, _, cx| this.save_editor(cx))),
                        )
                    })
                    .child(
                        Button::new("editor-close")
                            .label(t!("common.close").to_string())
                            .on_click(cx.listener(|this, _, _, cx| this.close_editor(cx))),
                    ),
            );

        let body = match editor {
            Some(state) => {
                let input = Input::new(&state).h_full().w_full().disabled(readonly);
                div()
                    .flex_1()
                    .min_h_0()
                    .border_1()
                    .border_color(rgb(CARD_BORDER))
                    .rounded_lg()
                    .child(input)
                    .into_any_element()
            }
            None => div().into_any_element(),
        };
        v_flex().size_full().p_4().gap_3().child(header).child(body)
    }
}

impl NyxApp {
    /// Switches the proxy mode (rule / global / direct) and reloads.
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

    /// Shows/hides the Home statistics sidebar.
    pub(crate) fn toggle_stats(&mut self, cx: &mut Context<Self>) {
        self.stats_open = !self.stats_open;
        cx.notify();
    }

    /// Persists a patch to the app config (used by Settings toggles).
    pub(crate) fn set_app_flag(&mut self, patch: serde_json::Value, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let _ = runtime::spawn(backend::config::patch_app_config(patch)).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    /// Opens a Settings sub-page, creating any text inputs it needs (prefilled from config).
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
                            InputState::new(window, cx)
                                .multi_line(true)
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
                            InputState::new(window, cx)
                                .multi_line(true)
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
                            InputState::new(window, cx)
                                .multi_line(true)
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
        self.sub_inputs = inputs;
        self.settings_sub = Some(sub);
        if matches!(sub, SettingsSub::Mihomo) {
            self.service_status = gpui::SharedString::default();
            self.core_version_installed = gpui::SharedString::default();
            self.refresh_service_info(cx);
        }
        if matches!(sub, SettingsSub::Resources) {
            self.proxy_providers.clear();
            self.rule_providers.clear();
            self.refresh_providers(cx);
        }
        cx.notify();
    }

    /// Returns from a Settings sub-page to the main settings list.
    pub(crate) fn close_settings_sub(&mut self, cx: &mut Context<Self>) {
        self.settings_sub = None;
        self.sub_inputs = SubInputs::default();
        self.recording_shortcut = None;
        cx.notify();
    }

    /// Begins recording a new binding for the given app-config shortcut key.
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

    /// Handles a keystroke while recording: Esc cancels, Backspace clears, any
    /// other combo is saved + re-registered.
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

    /// Persists a single shortcut binding (empty clears it) and reloads hotkeys.
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

    /// Persists the text fields of the active sub-page.
    pub(crate) fn save_settings_sub(&mut self, cx: &mut Context<Self>) {
        match self.settings_sub {
            Some(SettingsSub::Tun) => {
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
            Some(SettingsSub::SysProxy) => {
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
            Some(SettingsSub::Advanced) => {
                if let Some(n) = self
                    .sub_inputs
                    .interval
                    .as_ref()
                    .and_then(|e| e.read(cx).value().trim().parse::<u64>().ok())
                {
                    self.set_app_flag(serde_json::json!({ "networkDetectionInterval": n }), cx);
                }
            }
            Some(SettingsSub::Dns) => {
                let lines = |inp: &Option<Entity<InputState>>| -> Vec<String> {
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
            Some(SettingsSub::Mihomo) => {
                let num = |inp: &Option<Entity<InputState>>| -> Option<u64> {
                    inp.as_ref()
                        .and_then(|e| e.read(cx).value().trim().parse::<u64>().ok())
                };
                let lines = |inp: &Option<Entity<InputState>>| -> Vec<String> {
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
            Some(SettingsSub::Sniffer) => {
                let lines = |inp: &Option<Entity<InputState>>| -> Vec<String> {
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
            crate::app::actions::notify(
                gpui_component::notification::Notification::success(t!("common.saved")),
                cx,
            );
        });
    }

    /// Patches the controlled mihomo config under `sniffer` and restarts the core.
    pub(crate) fn patch_sniffer(&mut self, patch: serde_json::Value, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let body = serde_json::json!({ "sniffer": patch });
            let _ = runtime::spawn(backend::config::patch_controled_mihomo_config(body)).await;
            let _ = runtime::spawn(backend::manager::restart_core()).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    /// Re-fetches proxy + rule providers for the Resources page.
    pub(crate) fn refresh_providers(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let proxies = runtime::spawn(backend::api::get_proxy_providers()).await;
            let rules = runtime::spawn(backend::api::get_rule_providers()).await;
            let _ = this.update(cx, |this, cx| {
                if let Ok(Ok(v)) = proxies {
                    this.proxy_providers = parse_providers(&v, false);
                }
                if let Ok(Ok(v)) = rules {
                    this.rule_providers = parse_providers(&v, true);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Updates a single provider (proxy or rule) by name, then refreshes.
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

    /// Updates every provider of one kind (the "Update all" button), then refreshes.
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

    /// Opens the provider-content viewer (Resources page) and loads the content.
    pub(crate) fn open_provider_viewer(
        &mut self,
        name: String,
        is_rule: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let editor = cx.new(|cx| InputState::new(window, cx).code_editor("yaml"));
        self.provider_viewer = Some(ProviderViewerState {
            title: name.clone(),
            editor: editor.clone(),
        });
        cx.notify();
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

    /// Closes the provider-content viewer.
    pub(crate) fn close_provider_viewer(&mut self, cx: &mut Context<Self>) {
        self.provider_viewer = None;
        cx.notify();
    }

    /// Asks the core to re-download its geo databases via `PATCH /configs/geo`.
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
                let note = if ok {
                    gpui_component::notification::Notification::info(t!("pages.settings.resOk"))
                } else {
                    gpui_component::notification::Notification::error(t!("pages.settings.resFail"))
                };
                crate::app::actions::notify(note, cx);
            });
            let _ = this.update(cx, |this, cx| {
                this.resources_busy = false;
                cx.notify();
            });
        })
        .detach();
    }

    /// Patches top-level mihomo config keys (ports, allow-lan, …) and restarts the core.
    pub(crate) fn patch_core(&mut self, patch: serde_json::Value, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let _ = runtime::spawn(backend::config::patch_controled_mihomo_config(patch)).await;
            let _ = runtime::spawn(backend::manager::restart_core()).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    /// Fetches the Windows service status + installed core version (Mihomo page).
    pub(crate) fn refresh_service_info(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let status = runtime::spawn(backend::service::service_status()).await;
            let version = runtime::spawn(backend::manager::get_installed_version()).await;
            let _ = this.update(cx, |this, cx| {
                if let Ok(Ok(s)) = status {
                    this.service_status = s.into();
                }
                if let Ok(Ok(v)) = version {
                    this.core_version_installed = v.into();
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Runs a Windows service action, then refreshes runtime data + status.
    pub(crate) fn service_action(&mut self, action: &'static str, cx: &mut Context<Self>) {
        if self.service_busy {
            return;
        }
        self.service_busy = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let res = runtime::spawn(async move {
                match action {
                    "install" => backend::service::install_service().await,
                    "uninstall" => backend::service::uninstall_service().await,
                    "start" => backend::service::start_service().await,
                    "stop" => backend::service::stop_service().await,
                    "restart" => backend::service::restart_service().await,
                    _ => Ok(()),
                }
            })
            .await;
            if let Ok(Err(e)) = &res {
                log::warn!("[service] {action} failed: {e}");
                let msg = format!("{}: {e}", action);
                cx.update(|cx| {
                    crate::app::actions::notify(
                        gpui_component::notification::Notification::error(msg),
                        cx,
                    );
                });
            }
            if matches!(action, "stop" | "uninstall") && matches!(res, Ok(Ok(()))) {
                // Stopping/uninstalling kills the core — drop the stale state.
                cx.update(|cx| {
                    AppState::global(cx).update(cx, |st, c| {
                        st.set_core_status(crate::app::state::CoreStatus::Stopped, c);
                        st.set_tun_enabled(false, c);
                    });
                });
                crate::app::bootstrap::refresh_runtime_data(cx).await;
            } else if action == "install" && matches!(res, Ok(Ok(()))) {
                // Bring the core up (TUN stays off) so its version/groups populate.
                crate::app::bootstrap::start_core_disconnected(cx).await;
            } else {
                crate::app::bootstrap::refresh_runtime_data(cx).await;
            }
            let _ = this.update(cx, |this, cx| {
                this.service_busy = false;
                this.refresh_service_info(cx);
            });
        })
        .detach();
    }

    /// Grants TUN caps to the Nyx binary via `pkexec setcap` (Linux); effective next launch.
    #[cfg(target_os = "linux")]
    pub(crate) fn grant_tun(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let res = runtime::spawn(async { backend::elevation::grant_tun_caps() }).await;
            let note = match res {
                Ok(Ok(())) => gpui_component::notification::Notification::info(
                    t!("pages.settings.tunGrantedToast").to_string(),
                ),
                Ok(Err(e)) => gpui_component::notification::Notification::error(e),
                Err(_) => gpui_component::notification::Notification::error(
                    t!("pages.settings.tunGrantFailed").to_string(),
                ),
            };
            cx.update(|cx| crate::app::actions::notify(note, cx));
        })
        .detach();
    }

    /// Persists + (re)installs the core channel (`mihomo` stable / `mihomo-alpha`), then restarts.
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
                cx.update(|cx| {
                    crate::app::actions::notify(
                        gpui_component::notification::Notification::error(msg),
                        cx,
                    );
                });
            }
            let _ = runtime::spawn(backend::manager::restart_core()).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
            let _ = this.update(cx, |this, cx| {
                this.service_busy = false;
                this.refresh_service_info(cx);
            });
        })
        .detach();
    }

    /// Patches the controlled mihomo config under `tun` and restarts the core.
    pub(crate) fn patch_tun(&mut self, patch: serde_json::Value, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let body = serde_json::json!({ "tun": patch });
            let _ = runtime::spawn(backend::config::patch_controled_mihomo_config(body)).await;
            let _ = runtime::spawn(backend::manager::restart_core()).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    /// Flips an override flag (`controlDns`/`controlSniff`/`controlTun`) and rebuilds the core config.
    pub(crate) fn toggle_override(
        &mut self,
        key: &'static str,
        checked: bool,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |_this, cx| {
            let patch = serde_json::json!({ key: checked });
            let _ = runtime::spawn(backend::config::patch_app_config(patch)).await;
            let _ = runtime::spawn(backend::manager::restart_core()).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    /// Patches the controlled mihomo config under `dns` and restarts the core.
    pub(crate) fn patch_dns(&mut self, patch: serde_json::Value, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, cx| {
            let body = serde_json::json!({ "dns": patch });
            let _ = runtime::spawn(backend::config::patch_controled_mihomo_config(body)).await;
            let _ = runtime::spawn(backend::manager::restart_core()).await;
            crate::app::bootstrap::refresh_runtime_data(cx).await;
        })
        .detach();
    }

    /// Closes the connections with the given ids; the next `/connections` poll refreshes the list.
    pub(crate) fn close_connections(&mut self, ids: Vec<String>, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, _cx| {
            for id in ids {
                let _ =
                    runtime::spawn(async move { backend::api::close_connection(&id).await }).await;
            }
        })
        .detach();
    }

    /// Closes every active connection (the page-level "close all" button).
    pub(crate) fn close_all_connections(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |_this, _cx| {
            let _ = runtime::spawn(backend::api::close_all_connections()).await;
        })
        .detach();
    }
}
