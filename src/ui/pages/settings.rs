use gpui::base::input::{InputState, TextareaState};
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, Context, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Window, div, px,
};
use moonglass::theme::{ActiveTheme, Spec, ThemeMode};
use moonglass::{
    Button, ButtonSize, Dropdown, IconName, MenuItem, Section, Switch, Tabs, TextArea, TextField,
    W_SEMIBOLD, caption, h_flex, icon, motion, nav_row, setting_block, setting_row, v_flex,
};
use rust_i18n::t;

use crate::ui::root::{NyxApp, ProviderRow, SettingsSub};

const NAV_WIDTH: f32 = 240.;

const PAGES: [SettingsSub; 11] = [
    SettingsSub::General,
    SettingsSub::Appearance,
    SettingsSub::Mihomo,
    SettingsSub::Tun,
    SettingsSub::SysProxy,
    SettingsSub::Dns,
    SettingsSub::Sniffer,
    SettingsSub::Resources,
    SettingsSub::Advanced,
    SettingsSub::Shortcuts,
    SettingsSub::About,
];

fn page_title(sub: SettingsSub) -> String {
    match sub {
        SettingsSub::General => t!("pages.settings.general"),
        SettingsSub::Appearance => t!("pages.settings.appearance"),
        SettingsSub::Mihomo => t!("pages.settings.mihomo"),
        SettingsSub::Tun => t!("pages.settings.tunMode"),
        SettingsSub::SysProxy => t!("pages.settings.systemProxy"),
        SettingsSub::Dns => t!("pages.settings.dns"),
        SettingsSub::Sniffer => t!("pages.settings.sniffer"),
        SettingsSub::Resources => t!("pages.settings.resources"),
        SettingsSub::Advanced => t!("pages.settings.advanced"),
        SettingsSub::Shortcuts => t!("pages.settings.shortcuts"),
        SettingsSub::About => t!("pages.settings.about"),
    }
    .to_string()
}

/// A caption over a section of rows; `None` leaves the section untitled.
fn group(title: Option<String>, rows: Vec<AnyElement>, cx: &App) -> AnyElement {
    let section = rows
        .into_iter()
        .fold(Section::new(), |section, row| section.row(row));
    v_flex()
        .gap(px(6.))
        .children(title.map(|t| caption(t, cx)))
        .child(section)
        .into_any_element()
}

/// A muted line of help under a section.
fn note(text: String, cx: &App) -> AnyElement {
    div()
        .px(px(4.))
        .text_size(px(12.))
        .text_color(cx.theme().colors.muted)
        .line_height(gpui::relative(1.5))
        .child(text)
        .into_any_element()
}

/// Tabs on the inset pill the rest of the app uses for segmented choices.
fn pill(tabs: Tabs, cx: &App) -> AnyElement {
    let c = cx.theme().colors;
    h_flex()
        .flex_none()
        .p(px(2.))
        .rounded(px(8.))
        .border_1()
        .border_color(c.border)
        .bg(c.bg)
        .child(tabs)
        .into_any_element()
}

fn input_row(
    title: impl Into<SharedString>,
    input: Option<&Entity<InputState>>,
    enabled: bool,
    cx: &App,
) -> AnyElement {
    let control = match input {
        Some(state) => TextField::new(state)
            .disabled(!enabled)
            .w(px(260.))
            .into_any_element(),
        None => div().into_any_element(),
    };
    setting_row(title, None, control, cx).into_any_element()
}

fn list_block(
    title: impl Into<SharedString>,
    input: Option<&Entity<TextareaState>>,
    enabled: bool,
    cx: &App,
) -> AnyElement {
    let control = match input {
        Some(state) => TextArea::new(state).disabled(!enabled).into_any_element(),
        None => div().into_any_element(),
    };
    setting_block(title, None, control, cx).into_any_element()
}

impl NyxApp {
    pub(crate) fn render_settings(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let sub = self.settings_sub;
        let cards = match sub {
            SettingsSub::General => self.settings_general(cx),
            SettingsSub::Appearance => self.settings_appearance(cx),
            SettingsSub::Mihomo => self.settings_mihomo(cx),
            SettingsSub::Tun => self.settings_tun(cx),
            SettingsSub::SysProxy => self.settings_sysproxy(cx),
            SettingsSub::Dns => self.settings_dns(cx),
            SettingsSub::Sniffer => self.settings_sniffer(cx),
            SettingsSub::Resources => self.settings_resources(cx),
            SettingsSub::Advanced => self.settings_advanced(cx),
            SettingsSub::Shortcuts => self.settings_shortcuts(cx),
            SettingsSub::About => self.settings_about(window, cx),
        };
        let action = match sub {
            SettingsSub::Resources => Some(
                Button::new("res-geo")
                    .outline()
                    .icon(IconName::Download)
                    .label(t!("pages.settings.resUpdateGeo").to_string())
                    .loading(self.resources_busy)
                    .disabled(!self.state.read(cx).core_status.is_running())
                    .on_click(cx.listener(|this, _, _, cx| this.update_geo(cx))),
            ),
            _ => None,
        };
        let page_id = sub as usize;
        let title = motion::enter_nth(
            ("settings-title", page_id),
            h_flex()
                .gap(px(12.))
                .child(
                    div()
                        .flex_1()
                        .text_size(px(22.))
                        .font_weight(FontWeight::BOLD)
                        .child(page_title(sub)),
                )
                .children(action),
            6.,
            0,
            window,
            cx,
        );
        let cards: Vec<AnyElement> = cards
            .into_iter()
            .enumerate()
            .map(|(i, card)| {
                motion::enter_nth(
                    ("settings-card", page_id * 100 + i),
                    div().child(card),
                    10.,
                    i + 1,
                    window,
                    cx,
                )
            })
            .collect();
        let page = v_flex()
            .max_w(px(820.))
            .px(px(36.))
            .py(px(28.))
            .gap(px(22.))
            .child(title)
            .children(cards);
        let page =
            if sub == SettingsSub::Shortcuts {
                div()
                    .track_focus(&self.recorder_focus)
                    .on_key_down(cx.listener(|this, ev: &gpui::KeyDownEvent, _, cx| {
                        this.on_recorder_key(ev, cx)
                    }))
                    .child(page)
                    .into_any_element()
            } else {
                page.into_any_element()
            };
        let c = cx.theme().colors;
        let footer = self.settings_dirty.then(|| {
            h_flex()
                .flex_none()
                .gap(px(12.))
                .px(px(36.))
                .py(px(12.))
                .border_t_1()
                .border_color(c.border)
                .bg(c.panel)
                .child(icon(IconName::Alert, c.warn))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_color(c.text2)
                        .child(t!("pages.settings.unsaved").to_string()),
                )
                .child(
                    Button::new("settings-discard")
                        .ghost()
                        .label(t!("pages.settings.discard").to_string())
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.open_settings_sub(this.settings_sub, window, cx)
                        })),
                )
                .child(
                    Button::new("settings-save")
                        .primary()
                        .label(t!("common.save").to_string())
                        .on_click(cx.listener(|this, _, _, cx| this.save_settings_sub(cx))),
                )
        });
        h_flex()
            .size_full()
            .items_start()
            .child(self.render_settings_nav(window, cx))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .child(
                        div()
                            .id(("settings-page", page_id))
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .child(page),
                    )
                    .children(footer),
            )
    }

    fn render_settings_nav(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let c = cx.theme().colors;
        let back = nav_row("settings-back", false, window, cx)
            .mb(px(10.))
            .text_color(c.muted)
            .child(icon(IconName::ArrowLeft, c.muted))
            .child(t!("common.back").to_string())
            .on_click(cx.listener(|this, _, _, cx| this.close_settings(cx)));
        let rows: Vec<AnyElement> =
            PAGES
                .iter()
                .map(|&page| {
                    nav_row(
                        SharedString::from(format!("settings-page-{}", page as u8)),
                        page == self.settings_sub,
                        window,
                        cx,
                    )
                    .child(page_title(page))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_settings_sub(page, window, cx)
                    }))
                    .into_any_element()
                })
                .collect();
        let selected = PAGES.iter().position(|p| *p == self.settings_sub);
        let pages = motion::highlighted(
            "settings-nav",
            v_flex().gap(px(2.)),
            rows,
            selected,
            |d| d.rounded(px(6.)).bg(c.sel),
            window,
            cx,
        );
        v_flex()
            .w(px(NAV_WIDTH))
            .flex_none()
            .h_full()
            .bg(c.panel)
            .border_r_1()
            .border_color(c.border)
            .px(px(8.))
            .py(px(12.))
            .gap(px(2.))
            .child(back)
            .child(caption(t!("common.settings").to_string(), cx))
            .child(pages)
            .child(div().flex_1())
            .child(
                div()
                    .px(px(8.))
                    .font_family(cx.theme().mono.clone())
                    .text_size(px(11.))
                    .text_color(c.muted)
                    .child(format!(
                        "Nyx {} · {}",
                        env!("CARGO_PKG_VERSION"),
                        env!("NYX_REV")
                    )),
            )
    }

    /// A switch whose change runs `f` on the app; `enabled` greys it out.
    fn switch(
        &self,
        id: &'static str,
        checked: bool,
        enabled: bool,
        f: impl Fn(&mut Self, bool, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> Switch {
        let view = cx.entity().downgrade();
        Switch::new(id, checked)
            .disabled(!enabled)
            .on_change(move |on, window, cx| {
                let _ = view.update(cx, |this, cx| f(this, on, window, cx));
            })
    }

    #[allow(clippy::too_many_arguments)]
    fn switch_row(
        &self,
        id: &'static str,
        title: impl Into<SharedString>,
        hint: Option<String>,
        checked: bool,
        enabled: bool,
        f: impl Fn(&mut Self, bool, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let control = self.switch(id, checked, enabled, f, cx);
        setting_row(title, hint.map(Into::into), control, cx).into_any_element()
    }

    /// Tabs for a few named choices, writing the picked value through `f`.
    fn choice(
        &self,
        id: &'static str,
        options: Vec<(&'static str, String)>,
        current: &str,
        enabled: bool,
        f: impl Fn(&mut Self, &'static str, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let selected = options.iter().position(|(v, _)| *v == current).unwrap_or(0);
        let values: Vec<&'static str> = options.iter().map(|(v, _)| *v).collect();
        let view = cx.entity().downgrade();
        let tabs = Tabs::new(
            id,
            options.into_iter().map(|(_, l)| l.into()).collect(),
            selected,
            move |ix, _, cx| {
                if !enabled {
                    return;
                }
                let value = values[ix];
                let _ = view.update(cx, |this, cx| f(this, value, cx));
            },
        );
        div()
            .when(!enabled, |d| d.opacity(0.5))
            .child(pill(tabs, cx))
            .into_any_element()
    }

    /// The "override subscription" switch gating a DNS/Sniffer/TUN page.
    fn override_group(
        &self,
        id: &'static str,
        key: &'static str,
        value: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let row = self.switch_row(
            id,
            t!("pages.settings.overrideSettings"),
            Some(t!("pages.settings.overrideHint").to_string()),
            value,
            true,
            move |this, on, _, cx| this.toggle_override(key, on, cx),
            cx,
        );
        group(None, vec![row], cx)
    }

    fn settings_general(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let st = self.state.read(cx);
        let tun = st.tun_enabled;
        let sysproxy = st.app_flag("sysProxy.enable");
        let autostart = st.app_flag("autoStart");
        let silent = st.app_flag("silentStart");
        let language = st.language.clone();

        let connectivity = vec![
            self.switch_row(
                "set-tun",
                t!("pages.settings.tunMode"),
                None,
                tun,
                true,
                |this, _, _, cx| this.toggle_tun(cx),
                cx,
            ),
            self.switch_row(
                "set-sysproxy",
                t!("pages.settings.systemProxy"),
                None,
                sysproxy,
                true,
                |_, on, _, cx| crate::app::actions::set_sysproxy(on, cx),
                cx,
            ),
        ];
        let startup = vec![
            self.switch_row(
                "set-autostart",
                t!("pages.settings.autostart"),
                None,
                autostart,
                true,
                |this, on, _, cx| {
                    crate::app::autostart::set(on);
                    this.set_app_flag(serde_json::json!({ "autoStart": on }), cx)
                },
                cx,
            ),
            self.switch_row(
                "set-silent",
                t!("pages.settings.silentStart"),
                None,
                silent,
                true,
                |this, on, _, cx| this.set_app_flag(serde_json::json!({ "silentStart": on }), cx),
                cx,
            ),
        ];
        let view = cx.entity().downgrade();
        let languages = crate::app::state::LANGUAGES
            .iter()
            .map(|(code, name)| MenuItem::new(*code, *name))
            .collect();
        let language_row = setting_row(
            t!("common.language"),
            None,
            Dropdown::new("language", languages, Some(language), move |code, _, cx| {
                let _ = view.update(cx, |this, cx| this.select_language(&code, cx));
            })
            .width(px(180.)),
            cx,
        )
        .into_any_element();

        vec![
            v_flex()
                .gap(px(6.))
                .child(group(
                    Some(t!("pages.settings.connectivity").to_string()),
                    connectivity,
                    cx,
                ))
                .child(note(t!("pages.settings.connectivityHint").to_string(), cx))
                .into_any_element(),
            group(Some(t!("pages.settings.startup").to_string()), startup, cx),
            group(None, vec![language_row], cx),
        ]
    }

    fn settings_appearance(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let st = self.state.read(cx);
        let on_top = st.app_flag("alwaysOnTop");
        let disable_tray = st.app_flag("disableTray");
        let crate::ui::theme::Choice { mode, dark, light } = crate::ui::theme::choice(cx);

        let mode_row = setting_row(
            t!("pages.settings.themeMode"),
            None,
            self.choice(
                "theme-mode",
                vec![
                    ("system", t!("pages.settings.themeSystem").to_string()),
                    ("light", t!("pages.settings.themeLight").to_string()),
                    ("dark", t!("pages.settings.themeDark").to_string()),
                ],
                crate::ui::theme::mode_key(mode),
                true,
                |_, value, cx| {
                    let mode = match value {
                        "light" => ThemeMode::Light,
                        "dark" => ThemeMode::Dark,
                        _ => ThemeMode::System,
                    };
                    let choice = crate::ui::theme::Choice {
                        mode,
                        ..crate::ui::theme::choice(cx)
                    };
                    crate::ui::theme::choose(choice, cx);
                },
                cx,
            ),
            cx,
        )
        .into_any_element();
        let theme = cx.theme();
        let dark_list = theme.dark_themes.clone();
        let light_list = theme.light_themes.clone();
        let theme_rows = vec![
            mode_row,
            theme_row(
                t!("pages.settings.darkTheme").to_string(),
                &dark_list,
                &dark,
                true,
                cx,
            ),
            theme_row(
                t!("pages.settings.lightTheme").to_string(),
                &light_list,
                &light,
                false,
                cx,
            ),
        ];

        let window_rows = vec![
            self.switch_row(
                "ap-ontop",
                t!("pages.settings.alwaysOnTop"),
                None,
                on_top,
                true,
                |this, on, _, cx| this.set_app_flag(serde_json::json!({ "alwaysOnTop": on }), cx),
                cx,
            ),
            self.switch_row(
                "ap-tray",
                t!("pages.settings.disableTray"),
                None,
                disable_tray,
                true,
                |this, on, _, cx| this.set_app_flag(serde_json::json!({ "disableTray": on }), cx),
                cx,
            ),
        ];
        vec![
            group(None, theme_rows, cx),
            group(
                Some(t!("pages.settings.window").to_string()),
                window_rows,
                cx,
            ),
        ]
    }

    fn settings_mihomo(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let st = self.state.read(cx);
        let allow_lan = st.ctl_bool("allow-lan", false);
        let ipv6 = st.ctl_bool("ipv6", false);
        let unified_delay = st.ctl_bool("unified-delay", false);
        let tcp_concurrent = st.ctl_bool("tcp-concurrent", false);
        let disable_keep_alive = st.ctl_bool("disable-keep-alive", false);
        let store_selected = st.ctl_bool("profile.store-selected", false);
        let store_fake_ip = st.ctl_bool("profile.store-fake-ip", false);
        let str_of = |path: &str, default: &str| {
            st.ctl(path)
                .and_then(|v| v.as_str())
                .unwrap_or(default)
                .to_string()
        };
        let log_level = str_of("log-level", "info");
        let find_process = str_of("find-process-mode", "always");
        let fingerprint = str_of("global-client-fingerprint", "");
        let core = st
            .app_config
            .get("core")
            .and_then(|v| v.as_str())
            .unwrap_or("mihomo")
            .to_string();

        let core_toggle = |this: &Self,
                           id: &'static str,
                           title: String,
                           key: &'static str,
                           value: bool,
                           cx: &mut Context<Self>| {
            this.switch_row(
                id,
                title,
                None,
                value,
                true,
                move |this, on, _, cx| this.patch_core(serde_json::json!({ key: on }), cx),
                cx,
            )
        };
        let store_toggle = |this: &Self,
                            id: &'static str,
                            title: String,
                            key: &'static str,
                            value: bool,
                            cx: &mut Context<Self>| {
            this.switch_row(
                id,
                title,
                None,
                value,
                true,
                move |this, on, _, cx| {
                    this.patch_core(serde_json::json!({ "profile": { key: on } }), cx)
                },
                cx,
            )
        };

        let i = &self.sub_inputs;
        let ports = vec![
            input_row(
                t!("pages.settings.mixedPort"),
                i.mixed_port.as_ref(),
                true,
                cx,
            ),
            input_row(
                t!("pages.settings.socksPort"),
                i.socks_port.as_ref(),
                true,
                cx,
            ),
            input_row(
                t!("pages.settings.httpPort"),
                i.http_port.as_ref(),
                true,
                cx,
            ),
            input_row(
                t!("pages.settings.redirPort"),
                i.redir_port.as_ref(),
                true,
                cx,
            ),
            input_row(
                t!("pages.settings.tproxyPort"),
                i.tproxy_port.as_ref(),
                true,
                cx,
            ),
        ];
        let network = vec![
            core_toggle(
                self,
                "core-allowlan",
                t!("pages.settings.allowLan").to_string(),
                "allow-lan",
                allow_lan,
                cx,
            ),
            core_toggle(
                self,
                "core-ipv6",
                t!("pages.settings.coreIpv6").to_string(),
                "ipv6",
                ipv6,
                cx,
            ),
            core_toggle(
                self,
                "core-unified",
                t!("pages.settings.unifiedDelay").to_string(),
                "unified-delay",
                unified_delay,
                cx,
            ),
            core_toggle(
                self,
                "core-tcpconc",
                t!("pages.settings.tcpConcurrent").to_string(),
                "tcp-concurrent",
                tcp_concurrent,
                cx,
            ),
            core_toggle(
                self,
                "core-nokeepalive",
                t!("pages.settings.disableKeepAlive").to_string(),
                "disable-keep-alive",
                disable_keep_alive,
                cx,
            ),
            input_row(
                t!("pages.settings.keepAliveInterval"),
                i.keep_alive_interval.as_ref(),
                true,
                cx,
            ),
            input_row(
                t!("pages.settings.keepAliveIdle"),
                i.keep_alive_idle.as_ref(),
                true,
                cx,
            ),
            input_row(
                t!("pages.settings.interfaceName"),
                i.interface_name.as_ref(),
                true,
                cx,
            ),
        ];

        let view = cx.entity().downgrade();
        let dropdown = |id: &'static str,
                        key: &'static str,
                        options: Vec<(&'static str, String)>,
                        current: String| {
            let view = view.clone();
            Dropdown::new(
                id,
                options
                    .into_iter()
                    .map(|(v, l)| MenuItem::new(v, l))
                    .collect(),
                Some(current.into()),
                move |value, _, cx| {
                    let _ = view.update(cx, |this, cx| {
                        this.patch_core(serde_json::json!({ key: value.as_ref() }), cx)
                    });
                },
            )
            .width(px(180.))
        };
        let behavior = vec![
            setting_row(
                t!("pages.settings.logLevel"),
                None,
                dropdown(
                    "log-level",
                    "log-level",
                    vec![
                        ("info", "Info".into()),
                        ("warning", "Warning".into()),
                        ("error", "Error".into()),
                        ("debug", "Debug".into()),
                        ("silent", "Silent".into()),
                    ],
                    log_level,
                ),
                cx,
            )
            .into_any_element(),
            setting_row(
                t!("pages.settings.findProcess"),
                None,
                self.choice(
                    "find-process",
                    vec![
                        ("strict", t!("pages.settings.findProcessAuto").to_string()),
                        ("off", t!("pages.settings.findProcessOff").to_string()),
                        ("always", t!("pages.settings.findProcessOn").to_string()),
                    ],
                    &find_process,
                    true,
                    |this, value, cx| {
                        this.patch_core(serde_json::json!({ "find-process-mode": value }), cx)
                    },
                    cx,
                ),
                cx,
            )
            .into_any_element(),
            setting_row(
                t!("pages.settings.fingerprint"),
                None,
                dropdown(
                    "fingerprint",
                    "global-client-fingerprint",
                    vec![
                        ("", t!("pages.settings.fpDisabled").to_string()),
                        ("random", t!("pages.settings.fpRandom").to_string()),
                        ("chrome", "Chrome".into()),
                        ("firefox", "Firefox".into()),
                        ("safari", "Safari".into()),
                        ("ios", "iOS".into()),
                        ("android", "Android".into()),
                        ("edge", "Edge".into()),
                        ("360", "360".into()),
                        ("qq", "QQ".into()),
                    ],
                    fingerprint,
                ),
                cx,
            )
            .into_any_element(),
            store_toggle(
                self,
                "core-storesel",
                t!("pages.settings.storeSelected").to_string(),
                "store-selected",
                store_selected,
                cx,
            ),
            store_toggle(
                self,
                "core-storefakeip",
                t!("pages.settings.storeFakeIp").to_string(),
                "store-fake-ip",
                store_fake_ip,
                cx,
            ),
        ];
        let lists = vec![
            list_block(
                t!("pages.settings.skipAuthPrefixes"),
                i.skip_auth.as_ref(),
                true,
                cx,
            ),
            list_block(
                t!("pages.settings.lanAllowedIps"),
                i.lan_allowed.as_ref(),
                true,
                cx,
            ),
            list_block(
                t!("pages.settings.lanDisallowedIps"),
                i.lan_disallowed.as_ref(),
                true,
                cx,
            ),
        ];

        vec![
            self.core_section(&core, cx),
            self.service_section(cx),
            group(Some(t!("pages.settings.ports").to_string()), ports, cx),
            group(Some(t!("pages.settings.network").to_string()), network, cx),
            group(
                Some(t!("pages.settings.behavior").to_string()),
                behavior,
                cx,
            ),
            group(Some(t!("pages.settings.access").to_string()), lists, cx),
        ]
    }

    fn core_section(&self, core: &str, cx: &mut Context<Self>) -> AnyElement {
        let mono = cx.theme().mono.clone();
        let busy = self.service_busy;
        let installed = if self.core_version_installed.is_empty() {
            "…".to_string()
        } else {
            self.core_version_installed.to_string()
        };
        let running = self
            .state
            .read(cx)
            .mihomo_version
            .clone()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "—".to_string());
        let channel: &'static str = if core == "mihomo-alpha" {
            "mihomo-alpha"
        } else {
            "mihomo"
        };
        let value = |text: String| {
            div()
                .font_family(mono.clone())
                .text_size(px(12.))
                .text_color(cx.theme().colors.text2)
                .child(text)
        };
        let rows = vec![
            setting_row(
                t!("pages.settings.coreInstalled"),
                None,
                value(installed),
                cx,
            )
            .into_any_element(),
            setting_row(t!("pages.settings.coreRunning"), None, value(running), cx)
                .into_any_element(),
            setting_row(
                t!("pages.settings.coreChannel"),
                None,
                h_flex()
                    .gap(px(8.))
                    .child(self.choice(
                        "core-channel",
                        vec![
                            ("mihomo", t!("pages.settings.coreStable").to_string()),
                            (
                                "mihomo-alpha",
                                t!("pages.settings.corePrerelease").to_string(),
                            ),
                        ],
                        channel,
                        true,
                        |this, value, cx| {
                            this.set_app_flag(serde_json::json!({ "core": value }), cx)
                        },
                        cx,
                    ))
                    .child(
                        Button::new("core-update")
                            .primary()
                            .icon(IconName::Download)
                            .label(t!("pages.settings.coreUpdate").to_string())
                            .loading(busy)
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.install_core(channel, cx)),
                            ),
                    ),
                cx,
            )
            .into_any_element(),
        ];
        group(Some(t!("pages.settings.coreSection").to_string()), rows, cx)
    }

    fn service_section(&self, cx: &mut Context<Self>) -> AnyElement {
        let c = cx.theme().colors;
        let busy = self.service_busy;
        let status = self.service_status.to_string();
        let (label, color) = match status.as_str() {
            "running" => (t!("pages.settings.svcRunning"), c.ok),
            "stopped" => (t!("pages.settings.svcStopped"), c.warn),
            "stale" => (t!("pages.settings.svcStale"), c.warn),
            "not-installed" => (t!("pages.settings.svcNotInstalled"), c.muted),
            "" => (t!("pages.settings.svcChecking"), c.muted),
            _ => (t!("pages.settings.svcUnknown"), c.muted),
        };
        let stage = if self.service_managed {
            "managed"
        } else {
            status.as_str()
        };
        let actions: Vec<(&'static str, &'static str, String, bool)> = match stage {
            "running" => vec![
                (
                    "svc-restart",
                    "restart",
                    t!("pages.settings.svcRestart").to_string(),
                    false,
                ),
                (
                    "svc-stop",
                    "stop",
                    t!("pages.settings.svcStop").to_string(),
                    false,
                ),
                (
                    "svc-uninstall",
                    "uninstall",
                    t!("pages.settings.svcUninstall").to_string(),
                    true,
                ),
            ],
            "stopped" => vec![
                (
                    "svc-start",
                    "start",
                    t!("pages.settings.svcStart").to_string(),
                    false,
                ),
                (
                    "svc-uninstall",
                    "uninstall",
                    t!("pages.settings.svcUninstall").to_string(),
                    true,
                ),
            ],
            "managed" | "" => Vec::new(),
            "stale" => vec![(
                "svc-repair",
                "install",
                t!("pages.settings.svcRepair").to_string(),
                false,
            )],
            _ => vec![(
                "svc-install",
                "install",
                t!("pages.settings.svcInstall").to_string(),
                false,
            )],
        };
        let buttons = actions.into_iter().map(|(id, action, label, danger)| {
            let b = Button::new(id).label(label).disabled(busy);
            if danger { b.danger() } else { b.outline() }
                .on_click(cx.listener(move |this, _, _, cx| this.service_action(action, cx)))
        });
        let hint = if self.service_managed {
            t!("pages.settings.svcManagedHint").to_string()
        } else {
            t!("pages.settings.svcHint").to_string()
        };
        let hint = match self.service_detail.clone() {
            Some(reason) => format!("{reason}\n{hint}"),
            None => hint,
        };
        let status_row = setting_row(
            t!("pages.settings.svcStatus"),
            Some(hint.into()),
            h_flex()
                .gap(px(8.))
                .child(div().size(px(7.)).rounded(px(4.)).bg(color))
                .child(div().text_color(color).child(label.to_string())),
            cx,
        )
        .into_any_element();
        let mut rows = vec![status_row];
        let buttons: Vec<_> = buttons.collect();
        if !buttons.is_empty() {
            rows.push(
                h_flex()
                    .gap(px(8.))
                    .flex_wrap()
                    .px(px(18.))
                    .py(px(12.))
                    .children(buttons)
                    .into_any_element(),
            );
        }
        group(Some(t!("pages.settings.svcSection").to_string()), rows, cx)
    }

    fn settings_tun(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let st = self.state.read(cx);
        let enabled = st.tun_enabled;
        let stack = st
            .ctl("tun.stack")
            .and_then(|v| v.as_str())
            .unwrap_or("mixed")
            .to_string();
        let auto_route = st.ctl_bool("tun.auto-route", true);
        let strict = st.ctl_bool("tun.strict-route", false);
        let auto_detect = st.ctl_bool("tun.auto-detect-interface", true);
        let no_icmp = st.ctl_bool("tun.disable-icmp-forwarding", false);
        let on = st.app_flag("controlTun");

        let tun_toggle = |this: &Self,
                          id: &'static str,
                          title: String,
                          key: &'static str,
                          value: bool,
                          cx: &mut Context<Self>| {
            this.switch_row(
                id,
                title,
                None,
                value,
                on,
                move |this, checked, _, cx| this.patch_tun(serde_json::json!({ key: checked }), cx),
                cx,
            )
        };
        let main = vec![
            self.switch_row(
                "tun-enable",
                t!("pages.settings.tunTakeover"),
                None,
                enabled,
                on,
                |this, _, _, cx| this.toggle_tun(cx),
                cx,
            ),
            setting_row(
                t!("pages.settings.tunStack"),
                None,
                self.choice(
                    "tun-stack",
                    vec![
                        ("mixed", "mixed".into()),
                        ("system", "system".into()),
                        ("gvisor", "gvisor".into()),
                    ],
                    &stack,
                    on,
                    |this, value, cx| this.patch_tun(serde_json::json!({ "stack": value }), cx),
                    cx,
                ),
                cx,
            )
            .into_any_element(),
            input_row(
                t!("pages.settings.tunDevice"),
                self.sub_inputs.device.as_ref(),
                on,
                cx,
            ),
            input_row(
                t!("pages.settings.tunMtu"),
                self.sub_inputs.mtu.as_ref(),
                on,
                cx,
            ),
        ];
        let routing = vec![
            tun_toggle(
                self,
                "tun-autoroute",
                t!("pages.settings.tunAutoRoute").to_string(),
                "auto-route",
                auto_route,
                cx,
            ),
            tun_toggle(
                self,
                "tun-strict",
                t!("pages.settings.tunStrictRoute").to_string(),
                "strict-route",
                strict,
                cx,
            ),
            tun_toggle(
                self,
                "tun-autodetect",
                t!("pages.settings.tunAutoDetect").to_string(),
                "auto-detect-interface",
                auto_detect,
                cx,
            ),
            tun_toggle(
                self,
                "tun-noicmp",
                t!("pages.settings.tunNoIcmp").to_string(),
                "disable-icmp-forwarding",
                no_icmp,
                cx,
            ),
        ];
        vec![
            self.override_group("tun-override", "controlTun", on, cx),
            group(None, main, cx),
            group(None, routing, cx),
        ]
    }

    fn settings_sysproxy(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let st = self.state.read(cx);
        let enable = st.app_flag("sysProxy.enable");
        let affect_vpn = st.app_flag("affectVPNConnections");
        let mode = st
            .app_config
            .get("sysProxy")
            .and_then(|s| s.get("mode"))
            .and_then(|v| v.as_str())
            .unwrap_or("manual")
            .to_string();
        let rows = vec![
            self.switch_row(
                "sp-enable",
                t!("pages.settings.systemProxy"),
                None,
                enable,
                true,
                |_, on, _, cx| crate::app::actions::set_sysproxy(on, cx),
                cx,
            ),
            self.switch_row(
                "sp-vpn",
                t!("pages.settings.spAffectVpn"),
                None,
                affect_vpn,
                true,
                |this, on, _, cx| {
                    this.set_app_flag(serde_json::json!({ "affectVPNConnections": on }), cx)
                },
                cx,
            ),
            setting_row(
                t!("pages.settings.spMode"),
                None,
                self.choice(
                    "sp-mode",
                    vec![
                        ("manual", t!("pages.settings.spManual").to_string()),
                        ("pac", t!("pages.settings.spPac").to_string()),
                    ],
                    &mode,
                    true,
                    |this, value, cx| {
                        this.set_app_flag(serde_json::json!({ "sysProxy": { "mode": value } }), cx)
                    },
                    cx,
                ),
                cx,
            )
            .into_any_element(),
            input_row(
                t!("pages.settings.spHost"),
                self.sub_inputs.host.as_ref(),
                true,
                cx,
            ),
            input_row(
                t!("pages.settings.spBypass"),
                self.sub_inputs.bypass.as_ref(),
                true,
                cx,
            ),
        ];
        let mut cards = Vec::new();
        if sysproxy_partial() {
            let c = cx.theme().colors;
            cards.push(
                Section::new()
                    .row(
                        h_flex()
                            .items_start()
                            .gap(px(12.))
                            .px(px(18.))
                            .py(px(14.))
                            .child(icon(IconName::Alert, c.warn).mt(px(2.)))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_color(c.text2)
                                    .line_height(gpui::relative(1.5))
                                    .child(t!("pages.settings.sysProxyPartial").to_string()),
                            ),
                    )
                    .into_any_element(),
            );
        }
        cards.push(group(None, rows, cx));
        cards
    }

    fn settings_dns(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let st = self.state.read(cx);
        let enhanced = st
            .ctl("dns.enhanced-mode")
            .and_then(|v| v.as_str())
            .unwrap_or("fake-ip")
            .to_string();
        let ipv6 = st.ctl_bool("dns.ipv6", false);
        let respect_rules = st.ctl_bool("dns.respect-rules", false);
        let use_hosts = st.ctl_bool("dns.use-hosts", false);
        let use_sys_hosts = st.ctl_bool("dns.use-system-hosts", false);
        let on = st.app_flag("controlDns");

        let dns_toggle = |this: &Self,
                          id: &'static str,
                          title: String,
                          key: &'static str,
                          value: bool,
                          cx: &mut Context<Self>| {
            this.switch_row(
                id,
                title,
                None,
                value,
                on,
                move |this, checked, _, cx| this.patch_dns(serde_json::json!({ key: checked }), cx),
                cx,
            )
        };
        let i = &self.sub_inputs;
        let main = vec![
            setting_row(
                t!("pages.settings.dnsEnhancedMode"),
                None,
                self.choice(
                    "dns-mode",
                    vec![
                        ("fake-ip", "Fake-IP".into()),
                        ("redir-host", "Redir-Host".into()),
                    ],
                    &enhanced,
                    on,
                    |this, value, cx| {
                        this.patch_dns(serde_json::json!({ "enhanced-mode": value }), cx)
                    },
                    cx,
                ),
                cx,
            )
            .into_any_element(),
            input_row(
                t!("pages.settings.dnsFakeIpRange"),
                i.dns_fakeip_range.as_ref(),
                on,
                cx,
            ),
            dns_toggle(
                self,
                "dns-ipv6",
                t!("pages.settings.dnsIpv6").to_string(),
                "ipv6",
                ipv6,
                cx,
            ),
            dns_toggle(
                self,
                "dns-respect",
                t!("pages.settings.dnsRespectRules").to_string(),
                "respect-rules",
                respect_rules,
                cx,
            ),
            dns_toggle(
                self,
                "dns-hosts",
                t!("pages.settings.dnsUseHosts").to_string(),
                "use-hosts",
                use_hosts,
                cx,
            ),
            dns_toggle(
                self,
                "dns-syshosts",
                t!("pages.settings.dnsUseSystemHosts").to_string(),
                "use-system-hosts",
                use_sys_hosts,
                cx,
            ),
        ];
        let lists = vec![
            list_block(
                t!("pages.settings.dnsNameserver"),
                i.dns_nameserver.as_ref(),
                on,
                cx,
            ),
            list_block(
                t!("pages.settings.dnsDefaultNs"),
                i.dns_default_ns.as_ref(),
                on,
                cx,
            ),
            list_block(
                t!("pages.settings.dnsProxyNs"),
                i.dns_proxy_ns.as_ref(),
                on,
                cx,
            ),
            list_block(
                t!("pages.settings.dnsDirectNs"),
                i.dns_direct_ns.as_ref(),
                on,
                cx,
            ),
            list_block(
                t!("pages.settings.dnsFakeIpFilter"),
                i.dns_fakeip_filter.as_ref(),
                on,
                cx,
            ),
        ];
        vec![
            self.override_group("dns-override", "controlDns", on, cx),
            group(None, main, cx),
            group(None, lists, cx),
        ]
    }

    fn settings_sniffer(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let st = self.state.read(cx);
        let override_dest = st.ctl_bool("sniffer.override-destination", false);
        let force_dns = st.ctl_bool("sniffer.force-dns-mapping", true);
        let parse_pure_ip = st.ctl_bool("sniffer.parse-pure-ip", true);
        let sniff_http = st.ctl("sniffer.sniff.HTTP").is_some();
        let sniff_tls = st.ctl("sniffer.sniff.TLS").is_some();
        let sniff_quic = st.ctl("sniffer.sniff.QUIC").is_some();
        let on = st.app_flag("controlSniff");

        let toggle = |this: &Self,
                      id: &'static str,
                      title: String,
                      key: &'static str,
                      value: bool,
                      cx: &mut Context<Self>| {
            this.switch_row(
                id,
                title,
                None,
                value,
                on,
                move |this, checked, _, cx| {
                    this.patch_sniffer(serde_json::json!({ key: checked }), cx)
                },
                cx,
            )
        };
        // Per-protocol sniffing sets or clears `sniffer.sniff.<PROTO>.ports`.
        let proto = |this: &Self,
                     id: &'static str,
                     name: &'static str,
                     ports: &'static [&'static str],
                     value: bool,
                     cx: &mut Context<Self>| {
            this.switch_row(
                id,
                format!("{} {name}", t!("pages.settings.snifferSniff")),
                Some(ports.join(", ")),
                value,
                on,
                move |this, checked, _, cx| {
                    let entry = if checked {
                        serde_json::json!({ "ports": ports })
                    } else {
                        serde_json::Value::Null
                    };
                    this.patch_sniffer(serde_json::json!({ "sniff": { name: entry } }), cx)
                },
                cx,
            )
        };
        let main = vec![
            toggle(
                self,
                "sn-override",
                t!("pages.settings.snifferOverrideDest").to_string(),
                "override-destination",
                override_dest,
                cx,
            ),
            toggle(
                self,
                "sn-forcedns",
                t!("pages.settings.snifferForceDns").to_string(),
                "force-dns-mapping",
                force_dns,
                cx,
            ),
            toggle(
                self,
                "sn-purip",
                t!("pages.settings.snifferParsePureIp").to_string(),
                "parse-pure-ip",
                parse_pure_ip,
                cx,
            ),
        ];
        let protocols = vec![
            proto(
                self,
                "sn-http",
                "HTTP",
                &["80", "8080-8880"],
                sniff_http,
                cx,
            ),
            proto(self, "sn-tls", "TLS", &["443", "8443"], sniff_tls, cx),
            proto(self, "sn-quic", "QUIC", &["443", "8443"], sniff_quic, cx),
        ];
        let i = &self.sub_inputs;
        let lists = vec![
            list_block(
                t!("pages.settings.snifferForceDomain"),
                i.sniff_force_domain.as_ref(),
                on,
                cx,
            ),
            list_block(
                t!("pages.settings.snifferSkipDomain"),
                i.sniff_skip_domain.as_ref(),
                on,
                cx,
            ),
            list_block(
                t!("pages.settings.snifferSkipDst"),
                i.sniff_skip_dst.as_ref(),
                on,
                cx,
            ),
            list_block(
                t!("pages.settings.snifferSkipSrc"),
                i.sniff_skip_src.as_ref(),
                on,
                cx,
            ),
        ];
        vec![
            self.override_group("sniffer-override", "controlSniff", on, cx),
            group(None, main, cx),
            group(None, protocols, cx),
            group(None, lists, cx),
        ]
    }

    fn settings_resources(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        if !self.state.read(cx).core_status.is_running() {
            let c = cx.theme().colors;
            return vec![
                Section::new()
                    .row(
                        h_flex()
                            .items_start()
                            .gap(px(12.))
                            .px(px(18.))
                            .py(px(14.))
                            .child(icon(IconName::Info, c.accent).mt(px(2.)))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_color(c.text2)
                                    .line_height(gpui::relative(1.5))
                                    .child(t!("pages.settings.resCoreDown").to_string()),
                            ),
                    )
                    .into_any_element(),
            ];
        }
        vec![
            self.providers_section(
                t!("pages.settings.resProxyProviders").to_string(),
                self.proxy_providers.clone(),
                false,
                cx,
            ),
            self.providers_section(
                t!("pages.settings.resRuleProviders").to_string(),
                self.rule_providers.clone(),
                true,
                cx,
            ),
        ]
    }

    fn providers_section(
        &self,
        title: String,
        rows: Vec<ProviderRow>,
        is_rule: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = cx.theme().colors;
        let busy = self.resources_busy;
        let head = h_flex()
            .gap(px(8.))
            .child(div().flex_1().child(caption(title, cx)))
            .when(!rows.is_empty(), |row| {
                row.child(
                    Button::new(if is_rule {
                        "rule-prov-update-all"
                    } else {
                        "proxy-prov-update-all"
                    })
                    .ghost()
                    .size(ButtonSize::Xs)
                    .icon(IconName::Refresh)
                    .label(t!("pages.settings.resUpdateAll").to_string())
                    .disabled(busy)
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.update_all_providers(is_rule, cx)),
                    ),
                )
            });
        let section = if rows.is_empty() {
            Section::new().row(
                div()
                    .px(px(18.))
                    .py(px(14.))
                    .text_color(c.muted)
                    .child(t!("pages.settings.resNoProviders").to_string()),
            )
        } else {
            rows.into_iter().fold(Section::new(), |section, row| {
                let name = row.name.to_string();
                let view_name = name.clone();
                section.row(
                    h_flex()
                        .gap(px(12.))
                        .px(px(18.))
                        .py(px(10.))
                        .child(
                            v_flex()
                                .flex_1()
                                .min_w_0()
                                .child(div().truncate().font_weight(W_SEMIBOLD).child(row.name))
                                .child(
                                    div()
                                        .truncate()
                                        .font_family(cx.theme().mono.clone())
                                        .text_size(px(12.))
                                        .text_color(c.muted)
                                        .child(row.subtitle),
                                ),
                        )
                        .child(
                            Button::new(SharedString::from(format!("prov-view-{name}")))
                                .ghost()
                                .icon(IconName::Eye)
                                .tooltip(t!("pages.settings.resView").to_string())
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.open_provider_viewer(
                                        view_name.clone(),
                                        is_rule,
                                        window,
                                        cx,
                                    )
                                })),
                        )
                        .child(
                            Button::new(SharedString::from(format!("prov-{name}")))
                                .ghost()
                                .icon(IconName::Refresh)
                                .tooltip(t!("tooltips.update").to_string())
                                .disabled(busy)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.update_provider(name.clone(), is_rule, cx)
                                })),
                        ),
                )
            })
        };
        v_flex()
            .gap(px(6.))
            .child(head)
            .child(section)
            .into_any_element()
    }

    fn settings_advanced(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let st = self.state.read(cx);
        let stop_on_disconnect = st.app_flag("stopCoreOnDisconnect");
        let net_detect = st.app_flag("networkDetection");
        vec![
            group(
                None,
                vec![self.switch_row(
                    "ad-stopcore",
                    t!("pages.settings.stopCoreOnDisconnect"),
                    None,
                    stop_on_disconnect,
                    true,
                    |this, on, _, cx| {
                        this.set_app_flag(serde_json::json!({ "stopCoreOnDisconnect": on }), cx)
                    },
                    cx,
                )],
                cx,
            ),
            group(
                None,
                vec![
                    self.switch_row(
                        "ad-netdetect",
                        t!("pages.settings.networkDetection"),
                        None,
                        net_detect,
                        true,
                        |this, on, _, cx| {
                            this.set_app_flag(serde_json::json!({ "networkDetection": on }), cx)
                        },
                        cx,
                    ),
                    input_row(
                        t!("pages.settings.detectInterval"),
                        self.sub_inputs.interval.as_ref(),
                        true,
                        cx,
                    ),
                ],
                cx,
            ),
        ]
    }

    fn settings_shortcuts(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let c = cx.theme().colors;
        let mono = cx.theme().mono.clone();
        let cfg = self.state.read(cx).app_config.clone();
        let read = |key: &str| -> String {
            cfg.get(key)
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(crate::ui::pages::pretty_accel)
                .unwrap_or_else(|| "—".to_string())
        };
        let rows: [(&'static str, String); 7] = [
            (
                "showWindowShortcut",
                t!("pages.settings.scToggleWindow").to_string(),
            ),
            (
                "ruleModeShortcut",
                t!("pages.settings.scRuleMode").to_string(),
            ),
            (
                "globalModeShortcut",
                t!("pages.settings.scGlobalMode").to_string(),
            ),
            (
                "triggerTunShortcut",
                t!("pages.settings.scToggleTun").to_string(),
            ),
            (
                "triggerSysProxyShortcut",
                t!("pages.settings.scToggleSysproxy").to_string(),
            ),
            (
                "restartAppShortcut",
                t!("pages.settings.scRestart").to_string(),
            ),
            (
                "quitWithoutCoreShortcut",
                t!("pages.settings.scQuitKeepCore").to_string(),
            ),
        ];
        let key_chip = |text: String, active: bool| {
            div()
                .h(px(28.))
                .px(px(12.))
                .flex()
                .items_center()
                .rounded(px(6.))
                .border_1()
                .border_color(if active { c.accent } else { c.border })
                .bg(c.bg)
                .font_family(mono.clone())
                .text_size(px(12.))
                .text_color(if active { c.accent } else { c.text2 })
                .child(text)
        };

        // Wayland has no global-grab protocol, so we list the `nyx://` commands to bind instead.
        if !crate::app::hotkeys::supported() {
            let link = |key: &str| match key {
                "showWindowShortcut" => "nyx://toggle-window",
                "ruleModeShortcut" => "nyx://mode?value=rule",
                "globalModeShortcut" => "nyx://mode?value=global",
                "triggerTunShortcut" => "nyx://toggle-tun",
                "triggerSysProxyShortcut" => "nyx://toggle-sysproxy",
                "restartAppShortcut" => "nyx://restart",
                _ => "nyx://quit",
            };
            let items = rows
                .iter()
                .map(|(key, label)| {
                    setting_row(label.clone(), None, key_chip(link(key).into(), false), cx)
                        .into_any_element()
                })
                .collect();
            return vec![
                group(
                    Some(t!("pages.settings.scWaylandTitle").to_string()),
                    items,
                    cx,
                ),
                note(t!("pages.settings.scWaylandHint").to_string(), cx),
            ];
        }

        let items = rows
            .iter()
            .map(|(key, label)| {
                let key = *key;
                let recording = self.recording_shortcut == Some(key);
                let shown = if recording {
                    t!("pages.settings.scPress").to_string()
                } else {
                    read(key)
                };
                div()
                    .id(SharedString::from(format!("sc-{key}")))
                    .cursor_pointer()
                    .hover(|s| s.bg(c.row))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.start_recording_shortcut(key, window, cx)
                    }))
                    .child(setting_row(
                        label.clone(),
                        None,
                        key_chip(shown, recording),
                        cx,
                    ))
                    .into_any_element()
            })
            .collect();
        vec![
            group(None, items, cx),
            note(t!("pages.settings.scHint").to_string(), cx),
        ]
    }

    fn settings_about(&self, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let theme = cx.theme();
        let c = theme.colors;
        let mono = theme.mono.clone();
        let st = self.state.read(cx);
        let autocheck = st.app_flag("autoCheckUpdate");
        let core = st
            .mihomo_version
            .clone()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "—".to_string());
        let facts = build_facts(window, core);
        let report = facts
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>()
            .join("\n");
        let facts = facts
            .into_iter()
            .fold(Section::new(), |section, (label, value)| {
                section.row(
                    h_flex()
                        .gap(px(16.))
                        .px(px(18.))
                        .py(px(11.))
                        .child(div().flex_1().text_color(c.text2).child(label))
                        .child(
                            div()
                                .font_family(mono.clone())
                                .text_size(px(12.))
                                .truncate()
                                .child(value),
                        ),
                )
            });

        let (status, button) = match &self.update_info {
            Some(info) => (
                t!("pages.settings.updateAvailable", version => info.version).to_string(),
                t!("pages.settings.updateShow").to_string(),
            ),
            None if self.update_checking => (
                t!("updater.checking").to_string(),
                t!("pages.settings.checkUpdate").to_string(),
            ),
            None => (
                t!("pages.settings.updateThis", version => env!("CARGO_PKG_VERSION")).to_string(),
                t!("pages.settings.checkUpdate").to_string(),
            ),
        };
        let has_update = self.update_info.is_some();
        let updates = vec![
            setting_row(
                t!("pages.settings.updates"),
                Some(status.into()),
                Button::new("check-update")
                    .size(ButtonSize::Md)
                    .icon(IconName::Refresh)
                    .label(button)
                    .loading(self.update_checking)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if has_update {
                            this.open_updater(cx);
                        } else {
                            this.check_update(false, cx);
                        }
                    })),
                cx,
            )
            .into_any_element(),
            self.switch_row(
                "set-autocheck",
                t!("pages.settings.autoCheckUpdate"),
                None,
                autocheck,
                true,
                |this, on, _, cx| {
                    this.set_app_flag(serde_json::json!({ "autoCheckUpdate": on }), cx)
                },
                cx,
            ),
        ];
        let reset = setting_row(
            t!("pages.settings.resetApp"),
            Some(t!("pages.settings.resetWarning").trim().to_string().into()),
            Button::new("reset-app")
                .danger()
                .size(ButtonSize::Md)
                .label(t!("pages.settings.reset").to_string())
                .on_click(cx.listener(|this, _, _, cx| this.open_reset_confirm(cx))),
            cx,
        )
        .into_any_element();

        vec![
            h_flex()
                .gap(px(16.))
                .child(gpui::img("brand/logo.png").size(px(56.)).flex_none())
                .child(
                    v_flex()
                        .gap(px(2.))
                        .child(
                            div()
                                .text_size(px(18.))
                                .font_weight(FontWeight::BOLD)
                                .child("Nyx"),
                        )
                        .child(
                            div()
                                .text_color(c.muted)
                                .child(t!("pages.settings.tagline").to_string()),
                        ),
                )
                .into_any_element(),
            group(None, updates, cx),
            facts.into_any_element(),
            h_flex()
                .gap(px(8.))
                .child(
                    Button::new("source")
                        .size(ButtonSize::Md)
                        .icon(IconName::Code)
                        .label(t!("pages.settings.source").to_string())
                        .on_click(|_, _, cx| cx.open_url(env!("CARGO_PKG_REPOSITORY"))),
                )
                .child(
                    moonglass::copy_button("copy-info", move |_| report.clone(), cx)
                        .size(ButtonSize::Md)
                        .label(t!("pages.settings.copyInfo").to_string()),
                )
                .into_any_element(),
            group(None, vec![reset], cx),
        ]
    }
}

/// What a bug report needs to know about this build and machine.
fn build_facts(window: &Window, core: String) -> Vec<(String, String)> {
    let renderer = match window.gpu_specs() {
        Some(gpu) if gpu.is_software_emulated => {
            format!("{} ({})", gpu.device_name, t!("pages.settings.software"))
        }
        Some(gpu) => format!("{} · {}", gpu.device_name, gpu.driver_name),
        None if cfg!(target_os = "macos") => "Metal".into(),
        None if cfg!(target_os = "windows") => "Direct3D 11".into(),
        None => "wgpu".into(),
    };
    vec![
        (
            t!("pages.settings.appVersion").into(),
            env!("CARGO_PKG_VERSION").into(),
        ),
        (t!("pages.settings.commit").into(), env!("NYX_REV").into()),
        (
            t!("pages.settings.build").into(),
            if cfg!(debug_assertions) {
                "debug".into()
            } else {
                "release".into()
            },
        ),
        (t!("pages.settings.mihomo").into(), core),
        (
            t!("pages.settings.system").into(),
            format!("{} · {}", std::env::consts::OS, std::env::consts::ARCH),
        ),
        (
            t!("pages.settings.toolkit").into(),
            format!(
                "gpui-kit {} · moonglass {}",
                env!("NYX_GPUI_KIT"),
                moonglass::VERSION
            ),
        ),
        (t!("pages.settings.renderer").into(), renderer),
        (
            t!("pages.settings.license").into(),
            env!("CARGO_PKG_LICENSE").into(),
        ),
    ]
}

/// A theme choice: the picked theme's name under the title, and a swatch per theme to switch.
fn theme_row(title: String, list: &[Spec], current: &str, dark: bool, cx: &App) -> AnyElement {
    let c = cx.theme().colors;
    let name = list
        .iter()
        .find(|s| s.id == current)
        .or(list.first())
        .map(|s| s.name)
        .unwrap_or_default();
    let swatches = list.iter().map(|spec| {
        let p = spec.palette();
        let selected = spec.id == current;
        let id = spec.id;
        div()
            .id(SharedString::from(format!("theme-{id}")))
            .size(px(30.))
            .p(px(2.))
            .rounded(px(8.))
            .cursor_pointer()
            .bg(if selected { c.accent } else { c.border })
            .tooltip(moonglass::tooltip(spec.name.into()))
            .on_click(move |_, _, cx| {
                let mut choice = crate::ui::theme::choice(cx);
                if dark {
                    choice.dark = id.to_string();
                } else {
                    choice.light = id.to_string();
                }
                crate::ui::theme::choose(choice, cx);
            })
            .child(
                h_flex()
                    .size_full()
                    .rounded(px(6.))
                    .bg(p.bg)
                    .justify_center()
                    .child(div().size(px(10.)).rounded(px(5.)).bg(p.accent)),
            )
    });
    setting_row(
        title,
        Some(name.into()),
        h_flex().gap(px(6.)).children(swatches),
        cx,
    )
    .into_any_element()
}

/// True where the system proxy reaches only some apps, so TUN is the reliable option.
fn sysproxy_partial() -> bool {
    #[cfg(target_os = "linux")]
    {
        !crate::backend::sysproxy::session_honors_proxy()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}
