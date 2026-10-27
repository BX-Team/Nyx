use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, Context, IntoElement, ParentElement, SharedString, Styled, Window, div, px,
};
use moonglass::theme::ActiveTheme;
use moonglass::{Button, ButtonSize, IconName, Tabs, W_SEMIBOLD, h_flex, v_flex};
use rust_i18n::t;

use crate::app::actions::{MODE_SYSPROXY, MODE_TUN};
use crate::ui::root::{NyxApp, fmt_speed};

/// Up to two letters of a name for its tile: "Home VLESS" → "HV".
pub(crate) fn initials(name: &str) -> SharedString {
    let letters: String = name
        .split(|c: char| !c.is_alphanumeric())
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect();
    letters.to_uppercase().into()
}

fn fmt_elapsed(secs: u64) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

impl NyxApp {
    /// The bar under every screen: the profile and node in use, how traffic goes, and Connect.
    pub(crate) fn render_connection_bar(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = cx.theme().colors;
        let mono = cx.theme().mono.clone();
        let connection_mode = crate::app::actions::connection_mode(cx);
        let st = self.state.read(cx);
        let connected = st.tun_enabled || st.app_flag("sysProxy.enable");
        let global = st.mode.as_ref() == "global";
        let profile = st
            .current_profile_name
            .clone()
            .unwrap_or_else(|| SharedString::from("—"));
        let node = st
            .groups
            .iter()
            .find(|g| !g.now.is_empty())
            .map(|g| g.now.to_string());
        let (up, down) = (st.up_speed, st.down_speed);
        let failure = st
            .core_status
            .failed()
            .map(|(kind, _)| t!(crate::app::state::failure_key(kind)).to_string());
        let elapsed = self
            .connected_since
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0);

        let status = match failure {
            Some(text) => div()
                .text_color(moonglass::theme::danger())
                .truncate()
                .child(text)
                .into_any_element(),
            None if connected => h_flex()
                .gap(px(6.))
                .min_w_0()
                .child(div().size(px(7.)).rounded(px(4.)).bg(c.ok))
                .child(div().truncate().child(format!(
                    "{} · {} · ↓ {} ↑ {}",
                    t!("pages.home.connected"),
                    fmt_elapsed(elapsed),
                    fmt_speed(down),
                    fmt_speed(up)
                )))
                .into_any_element(),
            None => h_flex()
                .gap(px(6.))
                .child(
                    div()
                        .size(px(7.))
                        .rounded(px(4.))
                        .border_1()
                        .border_color(c.muted),
                )
                .child(t!("pages.home.disconnected").to_string())
                .into_any_element(),
        };

        let headline = h_flex()
            .min_w_0()
            .gap(px(6.))
            .font_weight(W_SEMIBOLD)
            .text_size(px(14.))
            .child(div().min_w_0().truncate().child(profile.clone()))
            .when_some(node, |row, node| {
                row.child(div().flex_none().text_color(c.muted).child("·"))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(crate::ui::flags::render_line(&node)),
                    )
            });

        let view = cx.entity().downgrade();
        let mode_tabs = Tabs::new(
            "bar-connection-mode",
            vec![
                t!("pages.home.modeTun").into(),
                t!("pages.home.modeSysProxy").into(),
            ],
            if connection_mode == MODE_TUN { 0 } else { 1 },
            {
                let view = view.clone();
                move |ix, _, cx| {
                    let mode = if ix == 0 { MODE_TUN } else { MODE_SYSPROXY };
                    let _ = view.update(cx, |this, cx| this.select_connection_mode(mode, cx));
                }
            },
        );
        let route_tabs = Tabs::new(
            "bar-proxy-mode",
            vec![t!("sider.modeRule").into(), t!("sider.modeGlobal").into()],
            usize::from(global),
            move |ix, _, cx| {
                let mode = if ix == 0 { "rule" } else { "global" };
                let _ = view.update(cx, |this, cx| this.set_proxy_mode(mode, cx));
            },
        );

        let button = if connected {
            Button::new("bar-disconnect")
                .outline()
                .size(ButtonSize::Lg)
                .icon(IconName::Power)
                .label(t!("pages.home.disconnect").to_string())
        } else {
            Button::new("bar-connect")
                .primary()
                .size(ButtonSize::Lg)
                .icon(IconName::Play)
                .label(t!("pages.home.connect").to_string())
        }
        .on_click(cx.listener(|this, _, _, cx| this.toggle_connection(cx)));

        h_flex()
            .flex_none()
            .min_h(px(72.))
            .gap(px(20.))
            .px(px(18.))
            .py(px(12.))
            .border_t_1()
            .border_color(c.border)
            .bg(c.panel)
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(12.))
                    .child(
                        moonglass::tile(crate::ui::bar::initials(&profile), 44., 9., cx)
                            .text_color(c.accent)
                            .text_size(px(14.)),
                    )
                    .child(
                        v_flex().min_w_0().gap(px(2.)).child(headline).child(
                            div()
                                .font_family(mono)
                                .text_size(px(12.))
                                .text_color(c.muted)
                                .child(status),
                        ),
                    ),
            )
            .child(
                h_flex()
                    .flex_none()
                    .gap(px(6.))
                    .p(px(2.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(c.border)
                    .bg(c.bg)
                    .child(mode_tabs),
            )
            .child(
                h_flex()
                    .flex_none()
                    .p(px(2.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(c.border)
                    .bg(c.bg)
                    .child(route_tabs),
            )
            .child(button)
            .into_any_element()
    }
}
