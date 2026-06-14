use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, rgb, rgba, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use gpui_component::{h_flex, input::Input, tooltip::Tooltip, v_flex, Icon, IconName, StyledExt};
use rust_i18n::t;

use crate::app::state::{ProxyGroup, ProxyNode};
use crate::ui::root::{
    delay_color, NyxApp, CARD_BG, CARD_BORDER, CONTROL_BG, CONTROL_BORDER, GREEN, GREEN_GLOW,
    GREEN_HI, MUTED, MUTED2, PANEL_BG, SUBTLE, TEXT,
};

/// Short uppercase label for a group's selection strategy.
fn kind_label(kind: &str) -> String {
    match kind {
        "Selector" => "SELECT".into(),
        "URLTest" => "URL-TEST".into(),
        "Fallback" => "FALLBACK".into(),
        "LoadBalance" => "BALANCE".into(),
        "Relay" => "RELAY".into(),
        other => other.to_uppercase(),
    }
}

impl NyxApp {
    pub(crate) fn render_proxies(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let groups = self.state.read(cx).groups.clone();

        let focused: Option<SharedString> = self
            .proxies_group
            .clone()
            .filter(|name| groups.iter().any(|g| &g.name == name))
            .or_else(|| groups.first().map(|g| g.name.clone()));

        let header = h_flex()
            .items_center()
            .justify_between()
            .px(px(22.))
            .pt(px(18.))
            .pb(px(16.))
            .child(
                div()
                    .text_xl()
                    .font_bold()
                    .text_color(rgb(TEXT))
                    .child(t!("sider.proxyGroup").to_string()),
            )
            .child({
                let f = focused.clone();
                let fu = focused.clone();
                h_flex()
                    .gap_2()
                    .items_center()
                    .when_some(fu, |row, name| {
                        row.child(
                            div()
                                .id("proxies-unfix")
                                .size(px(30.))
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
                                    Tooltip::new(t!("tooltips.unfix").to_string()).build(window, cx)
                                })
                                .child(Icon::empty().path("icons/refresh.svg").size(px(15.)))
                                .on_click(cx.listener(move |t, _, _, cx| {
                                    t.unfix_group(name.to_string(), cx)
                                })),
                        )
                    })
                    .child(
                        div()
                            .id("proxies-test")
                            .size(px(30.))
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
                                Tooltip::new(t!("tooltips.testLatency").to_string())
                                    .build(window, cx)
                            })
                            .child(Icon::empty().path("icons/activity.svg").size(px(15.)))
                            .when_some(f, |this, name| {
                                this.on_click(cx.listener(move |t, _, _, cx| {
                                    t.test_group_delay(name.to_string(), cx)
                                }))
                            }),
                    )
            });

        let body = if groups.is_empty() {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_color(rgb(MUTED))
                        .child(t!("pages.proxies.noGroups").to_string()),
                )
                .into_any_element()
        } else {
            // Group list as its own left panel, reading as a selector.
            let list = v_flex()
                .w(px(228.))
                .h_full()
                .flex_none()
                .p(px(6.))
                .gap(px(2.))
                .rounded(px(13.))
                .bg(rgb(PANEL_BG))
                .border_1()
                .border_color(rgb(CARD_BORDER))
                .id("proxies-list")
                .overflow_y_scroll()
                .children(groups.iter().map(|g| {
                    self.render_group_card(g, focused.as_deref() == Some(g.name.as_ref()), cx)
                }));

            let grid = match groups
                .iter()
                .find(|g| Some(g.name.as_ref()) == focused.as_deref())
            {
                Some(g) => self.render_node_grid(g, cx).into_any_element(),
                None => div().into_any_element(),
            };

            h_flex()
                .flex_1()
                .min_h_0()
                .items_start()
                .gap_4()
                .px(px(22.))
                .pb(px(18.))
                .child(list)
                .child(div().flex_1().min_w_0().h_full().child(grid))
                .into_any_element()
        };

        v_flex().size_full().child(header).child(body)
    }

    /// One entry in the left group list.
    fn render_group_card(
        &self,
        group: &ProxyGroup,
        active: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let name = group.name.clone();
        let now = group.now.clone();
        let count = group.all.len();
        let now_delay = group
            .all
            .iter()
            .find(|n| n.name == now)
            .and_then(|n| n.delay);

        let subtitle = if !now.is_empty() {
            match now_delay {
                Some(d) => format!("{now} · {d} ms"),
                None => now.to_string(),
            }
        } else {
            format!("{count} {}", t!("pages.proxies.nodes"))
        };
        let subtitle_color = if active && !now.is_empty() {
            GREEN_HI
        } else {
            MUTED2
        };

        v_flex()
            .id(SharedString::from(format!("grp-{name}")))
            .gap_1()
            .px(px(11.))
            .py(px(10.))
            .rounded(px(10.))
            .cursor_pointer()
            .when(active, |this| this.bg(rgba(GREEN_GLOW)))
            .when(!active, |this| this.hover(|s| s.bg(rgb(CARD_BG))))
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(rgb(TEXT))
                            .truncate()
                            .child(crate::ui::flags::render_name(&name)),
                    )
                    .child(kind_chip(group.kind.as_ref())),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(subtitle_color))
                    .truncate()
                    .child(crate::ui::flags::render_name(&subtitle)),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.proxies_group = Some(name.clone());
                cx.notify();
            }))
    }

    /// The 3-up node grid for the focused group.
    fn render_node_grid(&self, group: &ProxyGroup, cx: &mut Context<Self>) -> impl IntoElement {
        let group_name = group.name.to_string();
        let now = group.now.to_string();

        let query = self.proxies_search.read(cx).value().trim().to_lowercase();
        let alive_only = self.proxies_alive_only;
        let mut nodes: Vec<&ProxyNode> = group
            .all
            .iter()
            .filter(|n| query.is_empty() || n.name.to_lowercase().contains(&query))
            .filter(|n| !alive_only || n.delay.map(|d| d > 0).unwrap_or(false))
            .collect();
        if self.proxies_sort_latency {
            nodes.sort_by_key(|n| n.delay.unwrap_or(u32::MAX));
        }

        let mut col = v_flex().w_full().gap(px(11.));
        for chunk in nodes.chunks(3) {
            let mut row = h_flex().w_full().gap(px(11.)).items_stretch();
            for node in chunk {
                row = row.child(self.render_node(group_name.clone(), node, &now, cx));
            }
            // Pad short rows so cards keep a 1/3 width instead of stretching.
            for _ in chunk.len()..3 {
                row = row.child(div().flex_1());
            }
            col = col.child(row);
        }

        v_flex()
            .w_full()
            .h_full()
            .gap_3()
            .child(self.render_node_toolbar(cx))
            .child(
                v_flex()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .id("proxies-grid")
                    .overflow_y_scroll()
                    .child(col),
            )
    }

    /// Search box + sort-by-latency and alive-only toggles above the node grid.
    fn render_node_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let toggle = |id: &'static str, label: String, active: bool| {
            div()
                .id(SharedString::from(id))
                .px(px(11.))
                .py(px(6.))
                .rounded(px(8.))
                .text_xs()
                .cursor_pointer()
                .border_1()
                .when(active, |this| {
                    this.bg(rgb(GREEN))
                        .text_color(rgb(0x0B1014))
                        .border_color(rgb(GREEN))
                })
                .when(!active, |this| {
                    this.bg(rgb(CONTROL_BG))
                        .text_color(rgb(SUBTLE))
                        .border_color(rgb(CONTROL_BORDER))
                })
                .child(label)
        };

        h_flex()
            .w_full()
            .gap_2()
            .items_center()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(Input::new(&self.proxies_search)),
            )
            .child(
                toggle(
                    "px-sort",
                    t!("pages.proxies.sortByLatency").to_string(),
                    self.proxies_sort_latency,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.proxies_sort_latency = !this.proxies_sort_latency;
                    cx.notify();
                })),
            )
            .child(
                toggle(
                    "px-alive",
                    t!("pages.proxies.aliveOnly").to_string(),
                    self.proxies_alive_only,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.proxies_alive_only = !this.proxies_alive_only;
                    cx.notify();
                })),
            )
    }

    fn render_node(
        &self,
        group: String,
        node: &ProxyNode,
        now: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = node.name.as_ref() == now;
        let node_name = node.name.to_string();
        let test_group = group.clone();
        let test_node = node_name.clone();
        let delay_text = node
            .delay
            .map(|d| format!("{d} ms"))
            .unwrap_or_else(|| "—".to_string());

        v_flex()
            .id(SharedString::from(format!("node-{group}-{node_name}")))
            .relative()
            .flex_1()
            .min_w_0()
            .gap_2()
            .p(px(14.))
            .rounded(px(12.))
            .cursor_pointer()
            .bg(rgb(if selected { 0x13202A } else { CARD_BG }))
            .border_1()
            .border_color(rgb(if selected { 0x2F4A44 } else { CARD_BORDER }))
            .when(selected, |this| {
                this.child(
                    div()
                        .absolute()
                        .top(px(10.))
                        .right(px(10.))
                        .size(px(16.))
                        .rounded_full()
                        .bg(rgb(GREEN))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            Icon::new(IconName::Check)
                                .size(px(10.))
                                .text_color(rgb(0x0B1014)),
                        ),
                )
            })
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .text_color(rgb(TEXT))
                    .truncate()
                    .pr(px(18.))
                    .child(crate::ui::flags::render_name(&node_name)),
            )
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(delay_color(node.delay)))
                            .child(delay_text),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("test-{group}-{node_name}")))
                            .size(px(24.))
                            .rounded(px(7.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(rgb(CONTROL_BG))
                            .border_1()
                            .border_color(rgb(CONTROL_BORDER))
                            .text_color(rgb(SUBTLE))
                            .cursor_pointer()
                            .hover(|s| s.text_color(rgb(GREEN_HI)))
                            .tooltip(|window, cx| {
                                Tooltip::new(t!("tooltips.testLatency").to_string())
                                    .build(window, cx)
                            })
                            .child(Icon::empty().path("icons/activity.svg").size(px(13.)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.test_proxy_delay(test_group.clone(), test_node.clone(), cx);
                            })),
                    ),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.change_proxy(group.clone(), node_name.clone(), cx);
            }))
    }
}

/// A group's kind chip (`SELECT`, `URL-TEST`, …).
fn kind_chip(kind: &str) -> impl IntoElement {
    div()
        .px(px(6.))
        .py(px(2.))
        .rounded(px(5.))
        .bg(rgb(0x1A2530))
        .text_color(rgb(SUBTLE))
        .text_xs()
        .child(kind_label(kind))
}

fn seg_pill(label: &str, active: bool) -> impl IntoElement {
    div()
        .px(px(12.))
        .py(px(5.))
        .rounded(px(6.))
        .text_xs()
        .when(active, |this| this.bg(rgb(GREEN)).text_color(rgb(0x0B1014)))
        .when(!active, |this| this.text_color(rgb(MUTED)))
        .child(label.to_string())
}
