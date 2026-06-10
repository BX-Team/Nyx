use gpui::prelude::FluentBuilder;
use gpui::{
    div, img, px, rgb, rgba, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled,
};
use gpui_component::{h_flex, v_flex, Icon, IconName, StyledExt};
use rust_i18n::t;

use crate::ui::root::{NyxApp, Route};
use crate::ui::theme::*;

/// Off-state icon tint used throughout the rail (design `#74879a`).
const RAIL_ICON: u32 = 0x74879A;
const RAIL_W_COLLAPSED: f32 = 56.;
const RAIL_W_EXPANDED: f32 = 208.;

impl NyxApp {
    pub(crate) fn render_rail(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let expanded = self.rail_expanded;
        let mode = self.state.read(cx).mode.clone();

        let nav = v_flex().w_full().items_center().gap(px(2.)).children([
            self.rail_nav(
                "home",
                IconName::LayoutDashboard,
                t!("sider.home"),
                Route::Home,
                cx,
            ),
            self.rail_nav(
                "profiles",
                IconName::Folder,
                t!("sider.profileManagement"),
                Route::Profiles,
                cx,
            ),
            self.rail_nav(
                "proxies",
                IconName::Globe,
                t!("sider.proxyGroup"),
                Route::Proxies,
                cx,
            ),
            self.rail_nav(
                "rules",
                IconName::BookOpen,
                t!("sider.rules"),
                Route::Rules,
                cx,
            ),
            self.rail_nav(
                "conns",
                IconName::Network,
                t!("sider.connection"),
                Route::Connections,
                cx,
            ),
            self.rail_nav(
                "logs",
                IconName::SquareTerminal,
                t!("sider.logs"),
                Route::Logs,
                cx,
            ),
            self.rail_nav(
                "settings",
                IconName::Settings,
                t!("common.settings"),
                Route::Settings,
                cx,
            ),
        ]);

        let top = v_flex()
            .w_full()
            .items_center()
            .gap(px(8.))
            .child(self.brand_mark())
            .child(nav);

        let toggle_icon = if expanded {
            IconName::PanelLeftClose
        } else {
            IconName::PanelLeft
        };
        let bottom = v_flex().w_full().items_center().gap(px(2.)).children([
            self.rail_action(
                "rule",
                Icon::empty().path("icons/route.svg"),
                t!("sider.modeRule"),
                mode.as_ref() == "rule",
                cx.listener(|this, _, _, cx| this.set_proxy_mode("rule", cx)),
            ),
            self.rail_action(
                "global",
                Icon::new(IconName::Globe),
                t!("sider.modeGlobal"),
                mode.as_ref() == "global",
                cx.listener(|this, _, _, cx| this.set_proxy_mode("global", cx)),
            ),
            self.rail_action(
                "toggle",
                Icon::new(toggle_icon),
                t!("common.hideSidebar"),
                false,
                cx.listener(|this, _, _, cx| {
                    this.rail_expanded = !this.rail_expanded;
                    cx.notify();
                }),
            ),
        ]);

        v_flex()
            .flex_none()
            .w(px(if expanded {
                RAIL_W_EXPANDED
            } else {
                RAIL_W_COLLAPSED
            }))
            .h_full()
            .items_center()
            .justify_between()
            .px(px(8.))
            .pt(px(12.))
            .pb(px(14.))
            .bg(rgb(RAIL_BG))
            .border_r_1()
            .border_color(rgb(RAIL_BORDER))
            .child(top)
            .child(bottom)
    }

    /// The app logo (and wordmark, when expanded) in a dark rounded tile.
    fn brand_mark(&self) -> impl IntoElement {
        let logo = img("brand/logo.png").size(px(28.)).rounded(px(7.));
        if self.rail_expanded {
            h_flex()
                .w_full()
                .mb(px(2.))
                .px(px(6.))
                .gap_2()
                .items_center()
                .child(logo)
                .child(div().font_bold().text_color(rgb(TEXT)).child("Nyx"))
                .into_any_element()
        } else {
            div()
                .mb(px(2.))
                .flex()
                .items_center()
                .justify_center()
                .child(logo)
                .into_any_element()
        }
    }

    /// A primary destination (active when it matches the current route).
    fn rail_nav(
        &self,
        key: &str,
        icon: IconName,
        label: impl Into<SharedString>,
        route: Route,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let on = self.route == route;
        let on_click = cx.listener(move |this, _, _, cx| {
            // Always land on a page's root view, not wherever the user left it:
            // drop any open detail/sub-page state when switching destinations.
            this.editor_target = None;
            this.editor = None;
            this.rule_editor = None;
            this.settings_sub = None;
            this.sub_inputs = Default::default();
            this.recording_shortcut = None;
            this.conns_detail = None;
            this.conns_show_closed = false;
            this.conn_detail_item = None;
            this.proxies_group = None;
            this.route = route;
            cx.notify();
        });
        div()
            .id(SharedString::from(format!("rail-{key}")))
            .w_full()
            .child(self.rail_cell(Icon::new(icon), label, on))
            .on_click(on_click)
            .into_any_element()
    }

    /// A bottom action button (mode toggle / sidebar toggle).
    fn rail_action(
        &self,
        key: &str,
        icon: Icon,
        label: impl Into<SharedString>,
        on: bool,
        on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    ) -> gpui::AnyElement {
        div()
            .id(SharedString::from(format!("rail-act-{key}")))
            .w_full()
            .child(self.rail_cell(icon, label, on))
            .on_click(on_click)
            .into_any_element()
    }

    /// Shared rail cell: an icon-only square when collapsed, or a full-width
    /// icon+label row when expanded. Active state is a faint green wash — no
    /// heavy border or glow bar (kept deliberately light, like the old rail).
    fn rail_cell(&self, icon: Icon, label: impl Into<SharedString>, on: bool) -> impl IntoElement {
        let color = if on { GREEN_HI } else { RAIL_ICON };
        let glyph = icon.size(px(19.)).text_color(rgb(color));
        if self.rail_expanded {
            h_flex()
                .w_full()
                .h(px(38.))
                .px(px(10.))
                .gap_3()
                .items_center()
                .rounded(px(9.))
                .cursor_pointer()
                .when(on, |this| this.bg(rgba(GREEN_GLOW)))
                .child(glyph)
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(if on { TEXT } else { SUBTLE }))
                        .child(label.into()),
                )
                .into_any_element()
        } else {
            div()
                .size(px(38.))
                .rounded(px(9.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .when(on, |this| this.bg(rgba(GREEN_GLOW)))
                .child(glyph)
                .into_any_element()
        }
    }
}
