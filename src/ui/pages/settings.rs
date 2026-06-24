use gpui::prelude::FluentBuilder;
use gpui::Entity;
use gpui::{
    div, px, rgb, AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputState};
use gpui_component::select::Select;
use gpui_component::{
    h_flex, switch::Switch, tooltip::Tooltip, v_flex, Disableable, Icon, IconName, Sizable,
    StyledExt,
};
use rust_i18n::t;

use crate::ui::root::{
    NyxApp, SettingsSub, AMBER, CARD_BG, CARD_BORDER, CONTROL_BG, CONTROL_BORDER, DIVIDER, GREEN,
    GREEN_HI, MUTED3, RED, RED_HI, SUBTLE, TEXT,
};

impl NyxApp {
    pub(crate) fn render_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.settings_sub {
            Some(SettingsSub::Tun) => self.render_settings_tun(cx).into_any_element(),
            Some(SettingsSub::SysProxy) => self.render_settings_sysproxy(cx).into_any_element(),
            Some(SettingsSub::Dns) => self.render_settings_dns(cx).into_any_element(),
            Some(SettingsSub::Mihomo) => self.render_settings_mihomo(cx).into_any_element(),
            Some(SettingsSub::Sniffer) => self.render_settings_sniffer(cx).into_any_element(),
            Some(SettingsSub::Resources) => self.render_settings_resources(cx).into_any_element(),
            Some(SettingsSub::Appearance) => self.render_settings_appearance(cx).into_any_element(),
            Some(SettingsSub::Advanced) => self.render_settings_advanced(cx).into_any_element(),
            Some(SettingsSub::Shortcuts) => self.render_settings_shortcuts(cx).into_any_element(),
            None => self.render_settings_main(cx).into_any_element(),
        }
    }

    fn render_settings_main(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let st = self.state.read(cx);
        let tun = st.tun_enabled;
        let sysproxy = st.app_flag("sysProxy.enable");
        let autostart = st.app_flag("autoStart");
        let silent = st.app_flag("silentStart");
        let autocheck = st.app_flag("autoCheckUpdate");

        let connectivity = group(vec![
            toggle_row(
                t!("pages.settings.tunMode"),
                Some(self.gear("gear-tun", SettingsSub::Tun, cx)),
                Switch::new("set-tun")
                    .checked(tun)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_tun(cx))),
                false,
            ),
            toggle_row(
                t!("pages.settings.systemProxy"),
                Some(self.gear("gear-sysproxy", SettingsSub::SysProxy, cx)),
                Switch::new("set-sysproxy")
                    .checked(sysproxy)
                    .on_click(cx.listener(|_this, checked: &bool, _, cx| {
                        crate::app::actions::set_sysproxy(*checked, cx)
                    })),
                true,
            ),
        ]);

        let startup = group(vec![
            toggle_row(
                t!("pages.settings.autostart"),
                None,
                Switch::new("set-autostart")
                    .checked(autostart)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        crate::app::autostart::set(*checked);
                        this.set_app_flag(serde_json::json!({ "autoStart": *checked }), cx)
                    })),
                false,
            ),
            toggle_row(
                t!("pages.settings.silentStart"),
                None,
                Switch::new("set-silent")
                    .checked(silent)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.set_app_flag(serde_json::json!({ "silentStart": *checked }), cx)
                    })),
                false,
            ),
            toggle_row(
                t!("pages.settings.autoCheckUpdate"),
                None,
                Switch::new("set-autocheck")
                    .checked(autocheck)
                    .on_click(cx.listener(|this, checked: &bool, _, cx| {
                        this.set_app_flag(serde_json::json!({ "autoCheckUpdate": *checked }), cx)
                    })),
                true,
            ),
        ]);

        let language = group(vec![self.language_row()]);
        let sections = group(vec![
            self.nav_sub_row(t!("pages.settings.mihomo"), SettingsSub::Mihomo, false, cx),
            self.nav_sub_row(t!("pages.settings.dns"), SettingsSub::Dns, false, cx),
            self.nav_sub_row(
                t!("pages.settings.sniffer"),
                SettingsSub::Sniffer,
                false,
                cx,
            ),
            self.nav_sub_row(
                t!("pages.settings.resources"),
                SettingsSub::Resources,
                false,
                cx,
            ),
            self.nav_sub_row(
                t!("pages.settings.appearance"),
                SettingsSub::Appearance,
                false,
                cx,
            ),
            self.nav_sub_row(
                t!("pages.settings.advanced"),
                SettingsSub::Advanced,
                false,
                cx,
            ),
            self.nav_sub_row(
                t!("pages.settings.shortcuts"),
                SettingsSub::Shortcuts,
                true,
                cx,
            ),
        ]);

        settings_scroll(t!("pages.settings.title").to_string()).child(
            settings_body()
                .child(connectivity)
                .child(startup)
                .child(language)
                .child(sections)
                .child(group(vec![self.check_update_row(cx), version_row()]))
                .child(group(vec![self.reset_row(cx)])),
        )
    }

    /// The "Reset application" row — confirms before wiping all app data and relaunching.
    fn reset_row(&self, cx: &mut Context<Self>) -> AnyElement {
        row_shell(true)
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(TEXT))
                    .child(t!("pages.settings.resetApp").to_string()),
            )
            .child(
                div()
                    .id("reset-app-btn")
                    .h(px(32.))
                    .px(px(14.))
                    .flex()
                    .items_center()
                    .rounded(px(8.))
                    .bg(rgb(0x2A1614))
                    .border_1()
                    .border_color(rgb(RED))
                    .text_xs()
                    .text_color(rgb(RED_HI))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(0x3A1E1A)))
                    .child(t!("pages.settings.reset").to_string())
                    .on_click(cx.listener(|this, _, _, cx| this.open_reset_confirm(cx))),
            )
            .into_any_element()
    }

    fn render_settings_tun(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
        let override_on = st.app_flag("controlTun");

        let stack_seg = h_flex()
            .gap(px(2.))
            .p(px(3.))
            .rounded(px(9.))
            .bg(rgb(CONTROL_BG))
            .border_1()
            .border_color(rgb(CONTROL_BORDER))
            .children(["mixed", "system", "gvisor"].into_iter().map(|s| {
                let on = stack == s;
                div()
                    .id(SharedString::from(format!("tun-stack-{s}")))
                    .px(px(11.))
                    .py(px(4.))
                    .rounded(px(6.))
                    .text_xs()
                    .when(on, |t| t.bg(rgb(GREEN)).text_color(rgb(0x0B1014)))
                    .when(!on, |t| t.text_color(rgb(SUBTLE)))
                    .when(!override_on, |t| t.opacity(0.5))
                    .when(override_on, |t| {
                        t.cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.patch_tun(serde_json::json!({ "stack": s }), cx)
                            }))
                    })
                    .child(s)
            }));

        let body = settings_body()
            .child(self.override_group("tun-override", "controlTun", override_on, cx))
            .child(group(vec![
                toggle_row(
                    t!("pages.settings.tunTakeover"),
                    None,
                    Switch::new("tun-enable")
                        .checked(enabled)
                        .on_click(cx.listener(|this, _, _, cx| this.toggle_tun(cx))),
                    false,
                ),
                control_row(
                    t!("pages.settings.tunStack"),
                    stack_seg.into_any_element(),
                    false,
                ),
                input_row(
                    t!("pages.settings.tunDevice"),
                    self.sub_inputs.device.as_ref(),
                    override_on,
                    false,
                ),
                input_row(
                    t!("pages.settings.tunMtu"),
                    self.sub_inputs.mtu.as_ref(),
                    override_on,
                    true,
                ),
            ]))
            .child(group(vec![
                self.tun_toggle(
                    "tun-autoroute",
                    t!("pages.settings.tunAutoRoute"),
                    "auto-route",
                    auto_route,
                    override_on,
                    false,
                    cx,
                ),
                self.tun_toggle(
                    "tun-strict",
                    t!("pages.settings.tunStrictRoute"),
                    "strict-route",
                    strict,
                    override_on,
                    false,
                    cx,
                ),
                self.tun_toggle(
                    "tun-autodetect",
                    t!("pages.settings.tunAutoDetect"),
                    "auto-detect-interface",
                    auto_detect,
                    override_on,
                    false,
                    cx,
                ),
                self.tun_toggle(
                    "tun-noicmp",
                    t!("pages.settings.tunNoIcmp"),
                    "disable-icmp-forwarding",
                    no_icmp,
                    override_on,
                    true,
                    cx,
                ),
            ]));

        self.sub_scroll(
            t!("pages.settings.tunMode").to_string(),
            override_on,
            None,
            body,
            cx,
        )
    }

    /// One TUN boolean row that patches `tun.<key>` on toggle.
    #[allow(clippy::too_many_arguments)]
    fn tun_toggle(
        &self,
        id: &'static str,
        label: impl Into<SharedString>,
        key: &'static str,
        value: bool,
        enabled: bool,
        last: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        toggle_row(
            label,
            None,
            Switch::new(id)
                .checked(value)
                .disabled(!enabled)
                .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                    this.patch_tun(serde_json::json!({ key: *checked }), cx)
                })),
            last,
        )
    }

    fn render_settings_sysproxy(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

        let mode_seg = h_flex()
            .gap(px(2.))
            .p(px(3.))
            .rounded(px(9.))
            .bg(rgb(CONTROL_BG))
            .border_1()
            .border_color(rgb(CONTROL_BORDER))
            .children(["manual", "pac"].into_iter().map(|m| {
                let on = mode == m;
                div()
                    .id(SharedString::from(format!("sp-mode-{m}")))
                    .px(px(12.))
                    .py(px(4.))
                    .rounded(px(6.))
                    .text_xs()
                    .cursor_pointer()
                    .when(on, |t| t.bg(rgb(GREEN)).text_color(rgb(0x0B1014)))
                    .when(!on, |t| t.text_color(rgb(SUBTLE)))
                    .child(
                        if m == "manual" {
                            t!("pages.settings.spManual")
                        } else {
                            t!("pages.settings.spPac")
                        }
                        .to_string(),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_app_flag(serde_json::json!({ "sysProxy": { "mode": m } }), cx)
                    }))
            }));

        let body = settings_body()
            .when(sysproxy_partial(), |b| b.child(sysproxy_partial_note()))
            .child(group(vec![
                toggle_row(
                    t!("pages.settings.systemProxy"),
                    None,
                    Switch::new("sp-enable")
                        .checked(enable)
                        .on_click(cx.listener(|_this, checked: &bool, _, cx| {
                            crate::app::actions::set_sysproxy(*checked, cx)
                        })),
                    false,
                ),
                toggle_row(
                    t!("pages.settings.spAffectVpn"),
                    None,
                    Switch::new("sp-vpn")
                        .checked(affect_vpn)
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            this.set_app_flag(
                                serde_json::json!({ "affectVPNConnections": *checked }),
                                cx,
                            )
                        })),
                    false,
                ),
                control_row(
                    t!("pages.settings.spMode"),
                    mode_seg.into_any_element(),
                    false,
                ),
                input_row(
                    t!("pages.settings.spHost"),
                    self.sub_inputs.host.as_ref(),
                    true,
                    false,
                ),
                input_row(
                    t!("pages.settings.spBypass"),
                    self.sub_inputs.bypass.as_ref(),
                    true,
                    true,
                ),
            ]));

        self.sub_scroll(
            t!("pages.settings.systemProxy").to_string(),
            true,
            None,
            body,
            cx,
        )
    }

    fn render_settings_dns(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
        let override_on = st.app_flag("controlDns");

        let mode_seg = h_flex()
            .gap(px(2.))
            .p(px(3.))
            .rounded(px(9.))
            .bg(rgb(CONTROL_BG))
            .border_1()
            .border_color(rgb(CONTROL_BORDER))
            .children(
                [("fake-ip", "Fake-IP"), ("redir-host", "Redir-Host")]
                    .into_iter()
                    .map(|(m, label)| {
                        let on = enhanced == m;
                        div()
                            .id(SharedString::from(format!("dns-em-{m}")))
                            .px(px(11.))
                            .py(px(4.))
                            .rounded(px(6.))
                            .text_xs()
                            .when(on, |t| t.bg(rgb(GREEN)).text_color(rgb(0x0B1014)))
                            .when(!on, |t| t.text_color(rgb(SUBTLE)))
                            .when(!override_on, |t| t.opacity(0.5))
                            .when(override_on, |t| {
                                t.cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.patch_dns(
                                            serde_json::json!({ "enhanced-mode": m }),
                                            cx,
                                        )
                                    }))
                            })
                            .child(label)
                    }),
            );

        let body = settings_body()
            .child(self.override_group("dns-override", "controlDns", override_on, cx))
            .child(group(vec![
                control_row(
                    t!("pages.settings.dnsEnhancedMode"),
                    mode_seg.into_any_element(),
                    false,
                ),
                input_row(
                    t!("pages.settings.dnsFakeIpRange"),
                    self.sub_inputs.dns_fakeip_range.as_ref(),
                    override_on,
                    false,
                ),
                self.dns_toggle(
                    "dns-ipv6",
                    t!("pages.settings.dnsIpv6"),
                    "ipv6",
                    ipv6,
                    override_on,
                    false,
                    cx,
                ),
                self.dns_toggle(
                    "dns-respect",
                    t!("pages.settings.dnsRespectRules"),
                    "respect-rules",
                    respect_rules,
                    override_on,
                    false,
                    cx,
                ),
                self.dns_toggle(
                    "dns-hosts",
                    t!("pages.settings.dnsUseHosts"),
                    "use-hosts",
                    use_hosts,
                    override_on,
                    false,
                    cx,
                ),
                self.dns_toggle(
                    "dns-syshosts",
                    t!("pages.settings.dnsUseSystemHosts"),
                    "use-system-hosts",
                    use_sys_hosts,
                    override_on,
                    true,
                    cx,
                ),
            ]))
            .child(dns_list_card(
                t!("pages.settings.dnsNameserver"),
                self.sub_inputs.dns_nameserver.as_ref(),
                override_on,
            ))
            .child(dns_list_card(
                t!("pages.settings.dnsDefaultNs"),
                self.sub_inputs.dns_default_ns.as_ref(),
                override_on,
            ))
            .child(dns_list_card(
                t!("pages.settings.dnsProxyNs"),
                self.sub_inputs.dns_proxy_ns.as_ref(),
                override_on,
            ))
            .child(dns_list_card(
                t!("pages.settings.dnsDirectNs"),
                self.sub_inputs.dns_direct_ns.as_ref(),
                override_on,
            ))
            .child(dns_list_card(
                t!("pages.settings.dnsFakeIpFilter"),
                self.sub_inputs.dns_fakeip_filter.as_ref(),
                override_on,
            ));

        self.sub_scroll(
            t!("pages.settings.dns").to_string(),
            override_on,
            None,
            body,
            cx,
        )
    }

    /// One DNS boolean row that patches `dns.<key>` on toggle.
    #[allow(clippy::too_many_arguments)]
    fn dns_toggle(
        &self,
        id: &'static str,
        label: impl Into<SharedString>,
        key: &'static str,
        value: bool,
        enabled: bool,
        last: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        toggle_row(
            label,
            None,
            Switch::new(id)
                .checked(value)
                .disabled(!enabled)
                .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                    this.patch_dns(serde_json::json!({ key: *checked }), cx)
                })),
            last,
        )
    }

    fn render_settings_mihomo(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let st = self.state.read(cx);
        let allow_lan = st.ctl_bool("allow-lan", false);
        let ipv6 = st.ctl_bool("ipv6", false);
        let unified_delay = st.ctl_bool("unified-delay", false);
        let tcp_concurrent = st.ctl_bool("tcp-concurrent", false);
        let disable_keep_alive = st.ctl_bool("disable-keep-alive", false);
        let store_selected = st.ctl_bool("profile.store-selected", false);
        let store_fake_ip = st.ctl_bool("profile.store-fake-ip", false);
        let log_level = st
            .ctl("log-level")
            .and_then(|v| v.as_str())
            .unwrap_or("info")
            .to_string();
        let find_process = st
            .ctl("find-process-mode")
            .and_then(|v| v.as_str())
            .unwrap_or("always")
            .to_string();
        let fingerprint = st
            .ctl("global-client-fingerprint")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let core = st
            .app_config
            .get("core")
            .and_then(|v| v.as_str())
            .unwrap_or("mihomo")
            .to_string();

        let body = settings_body()
            .child(self.mihomo_core_card(&core, cx))
            .when(cfg!(windows), |b| b.child(self.mihomo_service_card(cx)))
            .when(!cfg!(windows), |b| b.child(self.tun_permission_card(cx)))
            .child(group(vec![
                input_row(
                    t!("pages.settings.mixedPort"),
                    self.sub_inputs.mixed_port.as_ref(),
                    true,
                    false,
                ),
                input_row(
                    t!("pages.settings.socksPort"),
                    self.sub_inputs.socks_port.as_ref(),
                    true,
                    false,
                ),
                input_row(
                    t!("pages.settings.httpPort"),
                    self.sub_inputs.http_port.as_ref(),
                    true,
                    false,
                ),
                input_row(
                    t!("pages.settings.redirPort"),
                    self.sub_inputs.redir_port.as_ref(),
                    true,
                    false,
                ),
                input_row(
                    t!("pages.settings.tproxyPort"),
                    self.sub_inputs.tproxy_port.as_ref(),
                    true,
                    true,
                ),
            ]))
            .child(group(vec![
                self.core_toggle(
                    "core-allowlan",
                    t!("pages.settings.allowLan"),
                    "allow-lan",
                    allow_lan,
                    false,
                    cx,
                ),
                self.core_toggle(
                    "core-ipv6",
                    t!("pages.settings.coreIpv6"),
                    "ipv6",
                    ipv6,
                    false,
                    cx,
                ),
                self.core_toggle(
                    "core-unified",
                    t!("pages.settings.unifiedDelay"),
                    "unified-delay",
                    unified_delay,
                    false,
                    cx,
                ),
                self.core_toggle(
                    "core-tcpconc",
                    t!("pages.settings.tcpConcurrent"),
                    "tcp-concurrent",
                    tcp_concurrent,
                    false,
                    cx,
                ),
                self.core_toggle(
                    "core-nokeepalive",
                    t!("pages.settings.disableKeepAlive"),
                    "disable-keep-alive",
                    disable_keep_alive,
                    true,
                    cx,
                ),
            ]))
            .child(group(vec![
                input_row(
                    t!("pages.settings.keepAliveInterval"),
                    self.sub_inputs.keep_alive_interval.as_ref(),
                    true,
                    false,
                ),
                input_row(
                    t!("pages.settings.keepAliveIdle"),
                    self.sub_inputs.keep_alive_idle.as_ref(),
                    true,
                    false,
                ),
                input_row(
                    t!("pages.settings.interfaceName"),
                    self.sub_inputs.interface_name.as_ref(),
                    true,
                    true,
                ),
            ]))
            .child(group(vec![
                self.profile_toggle(
                    "core-storesel",
                    t!("pages.settings.storeSelected"),
                    "store-selected",
                    store_selected,
                    false,
                    cx,
                ),
                self.profile_toggle(
                    "core-storefakeip",
                    t!("pages.settings.storeFakeIp"),
                    "store-fake-ip",
                    store_fake_ip,
                    true,
                    cx,
                ),
            ]))
            .child(self.core_choice_card(
                "log-level",
                t!("pages.settings.logLevel"),
                vec![
                    ("info", "Info".into()),
                    ("warning", "Warning".into()),
                    ("error", "Error".into()),
                    ("debug", "Debug".into()),
                    ("silent", "Silent".into()),
                ],
                log_level,
                cx,
            ))
            .child(self.core_choice_card(
                "find-process-mode",
                t!("pages.settings.findProcess"),
                vec![
                    ("strict", t!("pages.settings.findProcessAuto").to_string()),
                    ("off", t!("pages.settings.findProcessOff").to_string()),
                    ("always", t!("pages.settings.findProcessOn").to_string()),
                ],
                find_process,
                cx,
            ))
            .child(self.core_choice_card(
                "global-client-fingerprint",
                t!("pages.settings.fingerprint"),
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
                cx,
            ))
            .child(dns_list_card(
                t!("pages.settings.skipAuthPrefixes"),
                self.sub_inputs.skip_auth.as_ref(),
                true,
            ))
            .child(dns_list_card(
                t!("pages.settings.lanAllowedIps"),
                self.sub_inputs.lan_allowed.as_ref(),
                true,
            ))
            .child(dns_list_card(
                t!("pages.settings.lanDisallowedIps"),
                self.sub_inputs.lan_disallowed.as_ref(),
                true,
            ));

        self.sub_scroll(
            t!("pages.settings.mihomo").to_string(),
            true,
            None,
            body,
            cx,
        )
    }

    /// Core version + channel (stable/prerelease) + update button.
    fn mihomo_core_card(&self, core: &str, cx: &mut Context<Self>) -> AnyElement {
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

        let channel_seg = h_flex()
            .gap(px(2.))
            .p(px(3.))
            .rounded(px(9.))
            .bg(rgb(CONTROL_BG))
            .border_1()
            .border_color(rgb(CONTROL_BORDER))
            .children(
                [
                    ("mihomo", t!("pages.settings.coreStable").to_string()),
                    (
                        "mihomo-alpha",
                        t!("pages.settings.corePrerelease").to_string(),
                    ),
                ]
                .into_iter()
                .map(|(val, label)| {
                    let on = channel == val;
                    div()
                        .id(SharedString::from(format!("core-ch-{val}")))
                        .px(px(11.))
                        .py(px(4.))
                        .rounded(px(6.))
                        .text_xs()
                        .cursor_pointer()
                        .when(on, |t| t.bg(rgb(GREEN)).text_color(rgb(0x0B1014)))
                        .when(!on, |t| t.text_color(rgb(SUBTLE)))
                        .child(label)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.set_app_flag(serde_json::json!({ "core": val }), cx)
                        }))
                }),
            );

        settings_card(t!("pages.settings.coreSection"))
            .child(kv_text(t!("pages.settings.coreInstalled"), installed))
            .child(kv_text(t!("pages.settings.coreRunning"), running))
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(channel_seg)
                    .child(
                        Button::new("core-update")
                            .primary()
                            .small()
                            .label(t!("pages.settings.coreUpdate").to_string())
                            .disabled(busy)
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.install_core(channel, cx)),
                            ),
                    ),
            )
            .into_any_element()
    }

    /// Windows service status + install/start/stop/restart/uninstall controls.
    fn mihomo_service_card(&self, cx: &mut Context<Self>) -> AnyElement {
        let busy = self.service_busy;
        let status = self.service_status.to_string();
        let (label, color) = match status.as_str() {
            "running" => (t!("pages.settings.svcRunning"), GREEN_HI),
            "stopped" => (t!("pages.settings.svcStopped"), AMBER),
            "not-installed" => (t!("pages.settings.svcNotInstalled"), MUTED3),
            "" => (t!("pages.settings.svcChecking"), MUTED3),
            _ => (t!("pages.settings.svcUnknown"), MUTED3),
        };

        let svc_btn = |id: &'static str, action: &'static str, label: String, danger: bool| {
            let b = Button::new(id).small().label(label).disabled(busy);
            let b = if danger { b.danger() } else { b.primary() };
            b.on_click(cx.listener(move |this, _, _, cx| this.service_action(action, cx)))
        };

        let mut actions = h_flex().gap_2().flex_wrap();
        match status.as_str() {
            "running" => {
                actions = actions
                    .child(svc_btn(
                        "svc-restart",
                        "restart",
                        t!("pages.settings.svcRestart").to_string(),
                        false,
                    ))
                    .child(svc_btn(
                        "svc-stop",
                        "stop",
                        t!("pages.settings.svcStop").to_string(),
                        true,
                    ))
                    .child(svc_btn(
                        "svc-uninstall",
                        "uninstall",
                        t!("pages.settings.svcUninstall").to_string(),
                        true,
                    ));
            }
            "stopped" => {
                actions = actions
                    .child(svc_btn(
                        "svc-start",
                        "start",
                        t!("pages.settings.svcStart").to_string(),
                        false,
                    ))
                    .child(svc_btn(
                        "svc-uninstall",
                        "uninstall",
                        t!("pages.settings.svcUninstall").to_string(),
                        true,
                    ));
            }
            "" => {}
            _ => {
                actions = actions.child(svc_btn(
                    "svc-install",
                    "install",
                    t!("pages.settings.svcInstall").to_string(),
                    false,
                ));
            }
        }

        settings_card(t!("pages.settings.svcSection"))
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .py(px(2.))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child(t!("pages.settings.svcStatus").to_string()),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(div().size(px(7.)).rounded_full().bg(rgb(color)))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(color))
                                    .child(label.to_string()),
                            ),
                    ),
            )
            .child(actions)
            .into_any_element()
    }

    /// Non-Windows service-card replacement: TUN capability status + (Linux) a grant button.
    fn tun_permission_card(&self, cx: &mut Context<Self>) -> AnyElement {
        let granted = tun_granted();
        let nixos = tun_is_nixos();
        let (label, color) = if granted {
            (t!("pages.settings.tunGranted"), GREEN_HI)
        } else {
            (t!("pages.settings.tunNotGranted"), AMBER)
        };
        let hint = if cfg!(target_os = "macos") {
            t!("pages.settings.tunHintMac")
        } else if nixos {
            t!("pages.settings.tunHintNixos")
        } else {
            t!("pages.settings.tunHint")
        };

        let card = settings_card(t!("pages.settings.tunSection"))
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .py(px(2.))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child(t!("pages.settings.tunStatus").to_string()),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(div().size(px(7.)).rounded_full().bg(rgb(color)))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(color))
                                    .child(label.to_string()),
                            ),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED3))
                    .child(hint.to_string()),
            );

        #[cfg(target_os = "linux")]
        let card = card.when(!granted && !nixos, |c| {
            c.child(
                h_flex().child(
                    Button::new("tun-grant")
                        .small()
                        .primary()
                        .label(t!("pages.settings.tunGrant").to_string())
                        .on_click(cx.listener(|this, _, _, cx| this.grant_tun(cx))),
                ),
            )
        });
        #[cfg(not(target_os = "linux"))]
        let _ = cx;

        card.into_any_element()
    }

    /// One top-level mihomo boolean row that patches `<key>` and restarts core.
    fn core_toggle(
        &self,
        id: &'static str,
        label: impl Into<SharedString>,
        key: &'static str,
        value: bool,
        last: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        toggle_row(
            label,
            None,
            Switch::new(id).checked(value).on_click(cx.listener(
                move |this, checked: &bool, _, cx| {
                    this.patch_core(serde_json::json!({ key: *checked }), cx)
                },
            )),
            last,
        )
    }

    /// A boolean row under the config's `profile` map (`store-selected` / `store-fake-ip`).
    fn profile_toggle(
        &self,
        id: &'static str,
        label: impl Into<SharedString>,
        key: &'static str,
        value: bool,
        last: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        toggle_row(
            label,
            None,
            Switch::new(id).checked(value).on_click(cx.listener(
                move |this, checked: &bool, _, cx| {
                    this.patch_core(serde_json::json!({ "profile": { key: *checked } }), cx)
                },
            )),
            last,
        )
    }

    /// A titled card of choice pills that patch `key` to the picked value (empty clears).
    fn core_choice_card(
        &self,
        key: &'static str,
        title: impl Into<SharedString>,
        options: Vec<(&'static str, String)>,
        current: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pills = h_flex()
            .flex_wrap()
            .gap(px(6.))
            .children(options.into_iter().map(|(value, label)| {
                let on = current == value;
                div()
                    .id(SharedString::from(format!("choice-{key}-{value}")))
                    .px(px(11.))
                    .py(px(5.))
                    .rounded(px(7.))
                    .text_xs()
                    .cursor_pointer()
                    .bg(rgb(CONTROL_BG))
                    .border_1()
                    .border_color(rgb(CONTROL_BORDER))
                    .when(on, |t| {
                        t.bg(rgb(GREEN))
                            .border_color(rgb(GREEN))
                            .text_color(rgb(0x0B1014))
                    })
                    .when(!on, |t| t.text_color(rgb(SUBTLE)))
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.patch_core(serde_json::json!({ key: value }), cx)
                    }))
            }));
        v_flex()
            .w_full()
            .rounded(px(14.))
            .bg(rgb(CARD_BG))
            .border_1()
            .border_color(rgb(CARD_BORDER))
            .p(px(14.))
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .text_color(rgb(SUBTLE))
                    .child(title.into()),
            )
            .child(pills)
            .into_any_element()
    }

    fn render_settings_sniffer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let st = self.state.read(cx);
        let override_dest = st.ctl_bool("sniffer.override-destination", false);
        let force_dns = st.ctl_bool("sniffer.force-dns-mapping", true);
        let parse_pure_ip = st.ctl_bool("sniffer.parse-pure-ip", true);
        let sniff_http = st.ctl("sniffer.sniff.HTTP").is_some();
        let sniff_tls = st.ctl("sniffer.sniff.TLS").is_some();
        let sniff_quic = st.ctl("sniffer.sniff.QUIC").is_some();
        let override_on = st.app_flag("controlSniff");

        let body = settings_body()
            .child(self.override_group("sniffer-override", "controlSniff", override_on, cx))
            .child(group(vec![
                self.sniffer_toggle(
                    "sn-override",
                    t!("pages.settings.snifferOverrideDest"),
                    "override-destination",
                    override_dest,
                    override_on,
                    false,
                    cx,
                ),
                self.sniffer_toggle(
                    "sn-forcedns",
                    t!("pages.settings.snifferForceDns"),
                    "force-dns-mapping",
                    force_dns,
                    override_on,
                    false,
                    cx,
                ),
                self.sniffer_toggle(
                    "sn-purip",
                    t!("pages.settings.snifferParsePureIp"),
                    "parse-pure-ip",
                    parse_pure_ip,
                    override_on,
                    true,
                    cx,
                ),
            ]))
            .child(group(vec![
                self.sniff_proto_toggle(
                    "sn-http",
                    "HTTP",
                    &["80", "8080-8880"],
                    sniff_http,
                    override_on,
                    false,
                    cx,
                ),
                self.sniff_proto_toggle(
                    "sn-tls",
                    "TLS",
                    &["443", "8443"],
                    sniff_tls,
                    override_on,
                    false,
                    cx,
                ),
                self.sniff_proto_toggle(
                    "sn-quic",
                    "QUIC",
                    &["443", "8443"],
                    sniff_quic,
                    override_on,
                    true,
                    cx,
                ),
            ]))
            .child(dns_list_card(
                t!("pages.settings.snifferForceDomain"),
                self.sub_inputs.sniff_force_domain.as_ref(),
                override_on,
            ))
            .child(dns_list_card(
                t!("pages.settings.snifferSkipDomain"),
                self.sub_inputs.sniff_skip_domain.as_ref(),
                override_on,
            ))
            .child(dns_list_card(
                t!("pages.settings.snifferSkipDst"),
                self.sub_inputs.sniff_skip_dst.as_ref(),
                override_on,
            ))
            .child(dns_list_card(
                t!("pages.settings.snifferSkipSrc"),
                self.sub_inputs.sniff_skip_src.as_ref(),
                override_on,
            ));
        self.sub_scroll(
            t!("pages.settings.sniffer").to_string(),
            override_on,
            None,
            body,
            cx,
        )
    }

    /// One sniffer boolean row (patches `sniffer.<key>` + restarts core).
    #[allow(clippy::too_many_arguments)]
    fn sniffer_toggle(
        &self,
        id: &'static str,
        label: impl Into<SharedString>,
        key: &'static str,
        value: bool,
        enabled: bool,
        last: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        toggle_row(
            label,
            None,
            Switch::new(id)
                .checked(value)
                .disabled(!enabled)
                .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                    this.patch_sniffer(serde_json::json!({ key: *checked }), cx)
                })),
            last,
        )
    }

    /// Per-protocol sniff toggle: sets/clears `sniffer.sniff.<PROTO>.ports`.
    #[allow(clippy::too_many_arguments)]
    fn sniff_proto_toggle(
        &self,
        id: &'static str,
        proto: &'static str,
        ports: &'static [&'static str],
        value: bool,
        enabled: bool,
        last: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        toggle_row(
            format!("{} {proto}", t!("pages.settings.snifferSniff")),
            None,
            Switch::new(id)
                .checked(value)
                .disabled(!enabled)
                .on_click(cx.listener(move |this, checked: &bool, _, cx| {
                    let entry = if *checked {
                        serde_json::json!({ "ports": ports })
                    } else {
                        serde_json::Value::Null
                    };
                    this.patch_sniffer(serde_json::json!({ "sniff": { proto: entry } }), cx)
                })),
            last,
        )
    }

    fn render_settings_resources(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let busy = self.resources_busy;
        let geo_btn = Button::new("res-geo")
            .primary()
            .small()
            .label(t!("pages.settings.resUpdateGeo").to_string())
            .disabled(busy)
            .on_click(cx.listener(|this, _, _, cx| this.update_geo(cx)))
            .into_any_element();
        let body = settings_body()
            .child(self.providers_card(
                t!("pages.settings.resProxyProviders").to_string(),
                self.proxy_providers.clone(),
                false,
                cx,
            ))
            .child(self.providers_card(
                t!("pages.settings.resRuleProviders").to_string(),
                self.rule_providers.clone(),
                true,
                cx,
            ));
        self.sub_scroll(
            t!("pages.settings.resources").to_string(),
            false,
            Some(geo_btn),
            body,
            cx,
        )
    }

    /// A card listing providers (per-row view + update) with an "Update all" header button.
    fn providers_card(
        &self,
        title: String,
        rows: Vec<crate::ui::root::ProviderRow>,
        is_rule: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let busy = self.resources_busy;
        let has_rows = !rows.is_empty();
        let header = h_flex()
            .items_center()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .text_color(rgb(SUBTLE))
                    .child(title),
            )
            .when(has_rows, |this| {
                this.child(
                    Button::new(if is_rule {
                        "rule-prov-update-all"
                    } else {
                        "proxy-prov-update-all"
                    })
                    .small()
                    .label(t!("pages.settings.resUpdateAll").to_string())
                    .disabled(busy)
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.update_all_providers(is_rule, cx)),
                    ),
                )
            });

        let mut card = v_flex()
            .w_full()
            .rounded(px(14.))
            .bg(rgb(CARD_BG))
            .border_1()
            .border_color(rgb(CARD_BORDER))
            .p(px(14.))
            .gap_2()
            .child(header);

        if !has_rows {
            card = card.child(
                div()
                    .text_sm()
                    .text_color(rgb(MUTED3))
                    .py(px(2.))
                    .child(t!("pages.settings.resNoProviders").to_string()),
            );
            return card.into_any_element();
        }
        for row in rows {
            let name = row.name.to_string();
            let view_name = name.clone();
            card = card.child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .py(px(4.))
                    .child(
                        v_flex()
                            .min_w_0()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(TEXT))
                                    .truncate()
                                    .child(row.name.to_string()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(MUTED3))
                                    .truncate()
                                    .child(row.subtitle.to_string()),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .child(
                                Button::new(SharedString::from(format!("prov-view-{name}")))
                                    .small()
                                    .ghost()
                                    .icon(Icon::empty().path("icons/square-pen.svg"))
                                    .tooltip(t!("pages.settings.resView").to_string())
                                    .disabled(busy)
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
                                    .small()
                                    .icon(Icon::empty().path("icons/refresh.svg"))
                                    .disabled(busy)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.update_provider(name.clone(), is_rule, cx)
                                    })),
                            ),
                    ),
            );
        }
        card.into_any_element()
    }

    fn render_settings_appearance(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let st = self.state.read(cx);
        let on_top = st.app_flag("alwaysOnTop");
        let disable_tray = st.app_flag("disableTray");

        let body = settings_body().child(group(vec![
            self.flag_toggle(
                "ap-ontop",
                t!("pages.settings.alwaysOnTop"),
                "alwaysOnTop",
                on_top,
                false,
                cx,
            ),
            self.flag_toggle(
                "ap-tray",
                t!("pages.settings.disableTray"),
                "disableTray",
                disable_tray,
                false,
                cx,
            ),
        ]));
        self.sub_scroll(
            t!("pages.settings.appearance").to_string(),
            false,
            None,
            body,
            cx,
        )
    }

    fn render_settings_advanced(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let st = self.state.read(cx);
        let stop_on_disconnect = st.app_flag("stopCoreOnDisconnect");
        let net_detect = st.app_flag("networkDetection");

        let body = settings_body()
            .child(group(vec![self.flag_toggle(
                "ad-stopcore",
                t!("pages.settings.stopCoreOnDisconnect"),
                "stopCoreOnDisconnect",
                stop_on_disconnect,
                true,
                cx,
            )]))
            .child(group(vec![
                self.flag_toggle(
                    "ad-netdetect",
                    t!("pages.settings.networkDetection"),
                    "networkDetection",
                    net_detect,
                    false,
                    cx,
                ),
                input_row(
                    t!("pages.settings.detectInterval"),
                    self.sub_inputs.interval.as_ref(),
                    true,
                    true,
                ),
            ]));
        self.sub_scroll(
            t!("pages.settings.advanced").to_string(),
            true,
            None,
            body,
            cx,
        )
    }

    fn render_settings_shortcuts(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let st = self.state.read(cx);
        let cfg = st.app_config.clone();
        let read = |key: &str| -> String {
            cfg.get(key)
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(crate::ui::pages::pretty_accel)
                .unwrap_or_else(|| "—".to_string())
        };
        let rows: [(&'static str, SharedString); 7] = [
            (
                "showWindowShortcut",
                t!("pages.settings.scToggleWindow").into(),
            ),
            ("ruleModeShortcut", t!("pages.settings.scRuleMode").into()),
            (
                "globalModeShortcut",
                t!("pages.settings.scGlobalMode").into(),
            ),
            (
                "triggerTunShortcut",
                t!("pages.settings.scToggleTun").into(),
            ),
            (
                "triggerSysProxyShortcut",
                t!("pages.settings.scToggleSysproxy").into(),
            ),
            ("restartAppShortcut", t!("pages.settings.scRestart").into()),
            (
                "quitWithoutCoreShortcut",
                t!("pages.settings.scQuitKeepCore").into(),
            ),
        ];
        let n = rows.len();
        let card = group(
            rows.iter()
                .enumerate()
                .map(|(i, (key, label))| {
                    let recording = self.recording_shortcut == Some(*key);
                    self.shortcut_row(key, label.clone(), read(key), recording, i + 1 == n, cx)
                })
                .collect(),
        );
        let hint = div()
            .px(px(24.))
            .pb(px(8.))
            .text_xs()
            .text_color(rgb(MUTED3))
            .child(t!("pages.settings.scHint").to_string());

        // Key-capture surface: rows focus this, the next keystroke is recorded.
        let body = div()
            .track_focus(&self.recorder_focus)
            .child(settings_body().child(card).child(hint))
            .on_key_down(cx.listener(|this, ev: &gpui::KeyDownEvent, _window, cx| {
                this.on_recorder_key(ev, cx)
            }));
        self.sub_scroll(
            t!("pages.settings.shortcuts").to_string(),
            false,
            None,
            body,
            cx,
        )
    }

    /// One shortcut row: click to record, shows "Press keys…" while recording.
    fn shortcut_row(
        &self,
        key: &'static str,
        label: SharedString,
        binding: String,
        recording: bool,
        last: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let display = if recording {
            t!("pages.settings.scPress").to_string()
        } else {
            binding
        };
        let (chip_border, chip_fg) = if recording {
            (GREEN, GREEN)
        } else {
            (CONTROL_BORDER, SUBTLE)
        };
        row_shell(last)
            .id(SharedString::from(format!("sc-{key}")))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, window, cx| {
                this.start_recording_shortcut(key, window, cx)
            }))
            .child(div().text_sm().text_color(rgb(TEXT)).child(label))
            .child(
                div()
                    .h(px(28.))
                    .px(px(12.))
                    .flex()
                    .items_center()
                    .rounded(px(7.))
                    .bg(rgb(CONTROL_BG))
                    .border_1()
                    .border_color(rgb(chip_border))
                    .text_xs()
                    .text_color(rgb(chip_fg))
                    .child(display),
            )
            .into_any_element()
    }

    /// One app-config boolean row (patches the flat `<key>` on toggle).
    fn flag_toggle(
        &self,
        id: &'static str,
        label: impl Into<SharedString>,
        key: &'static str,
        value: bool,
        last: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        toggle_row(
            label,
            None,
            Switch::new(id).checked(value).on_click(cx.listener(
                move |this, checked: &bool, _, cx| {
                    this.set_app_flag(serde_json::json!({ key: *checked }), cx)
                },
            )),
            last,
        )
    }

    /// The "override subscription" toggle + hint atop the DNS/Sniffer/TUN pages;
    /// `key` is the gating flag (`controlDns`/`controlSniff`/`controlTun`).
    fn override_group(
        &self,
        id: &'static str,
        key: &'static str,
        value: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        v_flex()
            .w_full()
            .gap(px(6.))
            .child(group(vec![toggle_row(
                t!("pages.settings.overrideSettings"),
                None,
                Switch::new(id).checked(value).on_click(cx.listener(
                    move |this, checked: &bool, _, cx| this.toggle_override(key, *checked, cx),
                )),
                true,
            )]))
            .child(
                div()
                    .px(px(4.))
                    .text_xs()
                    .text_color(rgb(MUTED3))
                    .child(t!("pages.settings.overrideHint").to_string()),
            )
            .into_any_element()
    }

    /// A clickable gear icon that opens a Settings sub-page.
    fn gear(&self, id: &'static str, sub: SettingsSub, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id(id)
            .cursor_pointer()
            .text_color(rgb(MUTED3))
            .tooltip(|window, cx| {
                Tooltip::new(t!("tooltips.configure").to_string()).build(window, cx)
            })
            .child(Icon::new(IconName::Settings).size(px(14.)))
            .on_click(
                cx.listener(move |this, _, window, cx| this.open_settings_sub(sub, window, cx)),
            )
            .into_any_element()
    }

    /// A section row that navigates into a sub-page on click.
    fn nav_sub_row(
        &self,
        label: impl Into<SharedString>,
        sub: SettingsSub,
        last: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let label: SharedString = label.into();
        row_shell(last)
            .id(SharedString::from(format!("navsub-{label}")))
            .cursor_pointer()
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .text_color(rgb(SUBTLE))
                    .child(label),
            )
            .child(
                Icon::new(IconName::ChevronRight)
                    .size(px(16.))
                    .text_color(rgb(MUTED3)),
            )
            .on_click(
                cx.listener(move |this, _, window, cx| this.open_settings_sub(sub, window, cx)),
            )
            .into_any_element()
    }

    /// Wraps a sub-page body with a back header plus optional Save + action elements.
    fn sub_scroll(
        &self,
        title: String,
        save: bool,
        action: Option<AnyElement>,
        body: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        v_flex()
            .size_full()
            .id("settings-sub-scroll")
            .overflow_y_scroll()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .px(px(24.))
                    .pt(px(18.))
                    .pb(px(16.))
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .id("settings-back")
                                    .size(px(32.))
                                    .rounded(px(8.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(rgb(CARD_BG))
                                    .border_1()
                                    .border_color(rgb(CARD_BORDER))
                                    .text_color(rgb(SUBTLE))
                                    .cursor_pointer()
                                    .tooltip(|window, cx| {
                                        Tooltip::new(t!("common.back").to_string())
                                            .build(window, cx)
                                    })
                                    .child(Icon::new(IconName::ChevronLeft).size(px(16.)))
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.close_settings_sub(cx)),
                                    ),
                            )
                            .child(
                                div()
                                    .text_xl()
                                    .font_bold()
                                    .text_color(rgb(TEXT))
                                    .child(title),
                            ),
                    )
                    .when_some(action, |this, a| this.child(a))
                    .when(save, |this| {
                        this.child(
                            div()
                                .id("settings-save")
                                .h(px(32.))
                                .px(px(16.))
                                .flex()
                                .items_center()
                                .rounded(px(8.))
                                .bg(rgb(GREEN))
                                .text_color(rgb(0x0B1014))
                                .text_xs()
                                .font_semibold()
                                .cursor_pointer()
                                .child(t!("common.save").to_string())
                                .on_click(cx.listener(|this, _, _, cx| this.save_settings_sub(cx))),
                        )
                    }),
            )
            .child(body)
            .into_any_element()
    }

    /// The "Check for updates" row; shows a checking state while a GitHub check runs.
    fn check_update_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let checking = self.update_checking;
        let button_label = if checking {
            t!("updater.checking")
        } else {
            t!("pages.settings.check")
        };
        row_shell(false)
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(TEXT))
                    .child(t!("pages.settings.checkUpdate").to_string()),
            )
            .child(
                div()
                    .id("check-update-btn")
                    .h(px(32.))
                    .px(px(14.))
                    .flex()
                    .items_center()
                    .rounded(px(8.))
                    .bg(rgb(0x1A2530))
                    .border_1()
                    .border_color(rgb(CONTROL_BORDER))
                    .text_xs()
                    .text_color(rgb(SUBTLE))
                    .cursor_pointer()
                    .child(button_label.to_string())
                    .on_click(cx.listener(|this, _, _, cx| this.check_update(false, cx))),
            )
            .into_any_element()
    }

    /// The language selector row (a dropdown over the language list).
    fn language_row(&self) -> AnyElement {
        let control = div()
            .w(px(160.))
            .child(Select::new(&self.lang_select).small());
        control_row(t!("common.language"), control.into_any_element(), true)
    }
}

/// Header + scroll container for a settings list.
fn settings_scroll(title: String) -> gpui::Stateful<gpui::Div> {
    v_flex()
        .size_full()
        .id("settings-scroll")
        .overflow_y_scroll()
        .child(
            div().px(px(24.)).pt(px(18.)).pb(px(16.)).child(
                div()
                    .text_xl()
                    .font_bold()
                    .text_color(rgb(TEXT))
                    .child(title),
            ),
        )
}

/// The padded column that holds the setting cards (full width to the right edge).
fn settings_body() -> gpui::Div {
    v_flex().w_full().px(px(24.)).pb(px(22.)).gap(px(14.))
}

/// A rounded group card; children are the (already divider-bordered) rows.
fn group(rows: Vec<AnyElement>) -> impl IntoElement {
    v_flex()
        .w_full()
        .rounded(px(14.))
        .bg(rgb(CARD_BG))
        .border_1()
        .border_color(rgb(CARD_BORDER))
        .overflow_hidden()
        .children(rows)
}

/// Base row: label-left / control-right, with an optional bottom divider.
fn row_shell(last: bool) -> gpui::Div {
    h_flex()
        .items_center()
        .justify_between()
        .px(px(18.))
        .py(px(15.))
        .when(!last, |this| this.border_b_1().border_color(rgb(DIVIDER)))
}

/// A row with a plain text label and an arbitrary control on the right.
fn control_row(label: impl Into<SharedString>, control: AnyElement, last: bool) -> AnyElement {
    row_shell(last)
        .child(div().text_sm().text_color(rgb(TEXT)).child(label.into()))
        .child(control)
        .into_any_element()
}

fn toggle_row(
    label: impl Into<SharedString>,
    gear: Option<AnyElement>,
    switch: Switch,
    last: bool,
) -> AnyElement {
    row_shell(last)
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(div().text_sm().text_color(rgb(TEXT)).child(label.into()))
                .when_some(gear, |this, g| this.child(g)),
        )
        .child(switch)
        .into_any_element()
}

/// A row with a label and a text input on the right (empty if no input yet).
/// `enabled` greys out the input when an override toggle gates the page.
fn input_row(
    label: impl Into<SharedString>,
    input: Option<&Entity<InputState>>,
    enabled: bool,
    last: bool,
) -> AnyElement {
    let control = match input {
        Some(state) => Input::new(state)
            .w(px(260.))
            .disabled(!enabled)
            .into_any_element(),
        None => div().into_any_element(),
    };
    control_row(label, control, last)
}

/// A full-width DNS list card: a label header above a multi-line text input
/// (one server / entry per line). `enabled` greys out the input.
fn dns_list_card(
    label: impl Into<SharedString>,
    input: Option<&Entity<InputState>>,
    enabled: bool,
) -> AnyElement {
    let control = match input {
        Some(state) => Input::new(state)
            .w_full()
            .disabled(!enabled)
            .into_any_element(),
        None => div().into_any_element(),
    };
    v_flex()
        .w_full()
        .rounded(px(14.))
        .bg(rgb(CARD_BG))
        .border_1()
        .border_color(rgb(CARD_BORDER))
        .p(px(14.))
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_semibold()
                .text_color(rgb(SUBTLE))
                .child(label.into()),
        )
        .child(control)
        .into_any_element()
}

/// Whether the running process can give the core TUN access (Linux: holds
/// `CAP_NET_ADMIN`; elsewhere: running as root).
fn tun_granted() -> bool {
    #[cfg(target_os = "linux")]
    {
        crate::backend::elevation::has_net_admin()
    }
    #[cfg(not(target_os = "linux"))]
    {
        crate::backend::elevation::is_elevated()
    }
}

/// On NixOS, TUN caps come from `programs.nyx.tunMode` (declarative wrapper),
/// not a runtime `setcap` — so we swap the grant button for instructions.
fn tun_is_nixos() -> bool {
    #[cfg(target_os = "linux")]
    {
        crate::backend::elevation::is_nixos()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// True on Linux desktops where the system proxy only reaches some apps (not
/// GNOME-like); used to warn that TUN is the reliable full-coverage option.
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

fn sysproxy_partial_note() -> AnyElement {
    div()
        .mb(px(10.))
        .px(px(12.))
        .py(px(9.))
        .rounded(px(9.))
        .border_1()
        .border_color(rgb(AMBER))
        .text_xs()
        .text_color(rgb(AMBER))
        .child(t!("pages.settings.sysProxyPartial").to_string())
        .into_any_element()
}

fn settings_card(title: impl Into<SharedString>) -> gpui::Div {
    v_flex()
        .w_full()
        .rounded(px(14.))
        .bg(rgb(CARD_BG))
        .border_1()
        .border_color(rgb(CARD_BORDER))
        .p(px(14.))
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_semibold()
                .text_color(rgb(SUBTLE))
                .child(title.into()),
        )
}

/// A `label : value` info row (read-only).
fn kv_text(label: impl Into<SharedString>, value: String) -> AnyElement {
    h_flex()
        .items_center()
        .justify_between()
        .gap_2()
        .py(px(2.))
        .child(div().text_sm().text_color(rgb(TEXT)).child(label.into()))
        .child(
            div()
                .text_sm()
                .text_color(rgb(MUTED3))
                .truncate()
                .child(value),
        )
        .into_any_element()
}

/// The current installed app version, shown beneath the "check for updates" row.
fn version_row() -> AnyElement {
    row_shell(true)
        .child(
            div()
                .text_sm()
                .text_color(rgb(TEXT))
                .child(t!("pages.settings.appVersion").to_string()),
        )
        .child(
            div()
                .text_sm()
                .text_color(rgb(MUTED3))
                .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
        )
        .into_any_element()
}

#[allow(dead_code)]
fn action_row(
    label: impl Into<SharedString>,
    danger: bool,
    button_label: impl Into<SharedString>,
) -> AnyElement {
    let (bg, border, fg) = if danger {
        (0x2A1614, RED, RED_HI)
    } else {
        (0x1A2530, CONTROL_BORDER, SUBTLE)
    };
    row_shell(true)
        .child(div().text_sm().text_color(rgb(TEXT)).child(label.into()))
        .child(
            div()
                .h(px(32.))
                .px(px(14.))
                .flex()
                .items_center()
                .rounded(px(8.))
                .bg(rgb(bg))
                .border_1()
                .border_color(rgb(border))
                .text_xs()
                .text_color(rgb(fg))
                .child(button_label.into()),
        )
        .into_any_element()
}
