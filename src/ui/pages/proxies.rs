use gpui::base::Selectable;
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use moonglass::theme::ActiveTheme;
use moonglass::{
    Button, ButtonSize, IconName, Section, TextField, Tone, caption, chip, h_flex, icon, motion,
    placeholder, v_flex,
};
use rust_i18n::t;

use crate::app::state::{ProxyGroup, ProxyNode};
use crate::ui::pages::header;
use crate::ui::root::{NyxApp, delay_color};

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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let groups = self.state.read(cx).groups.clone();
        let focused: Option<SharedString> = self
            .proxies_group
            .clone()
            .filter(|name| groups.iter().any(|g| &g.name == name))
            .or_else(|| groups.first().map(|g| g.name.clone()));

        let toolbar = {
            let test = focused.clone();
            let unfix = focused.clone();
            header(
                t!("sider.proxyGroup").to_string(),
                Some(groups.len().to_string().into()),
                cx,
            )
            .child(
                TextField::new(&self.proxies_search)
                    .leading(IconName::Search)
                    .clearable()
                    .w(px(220.)),
            )
            .child(
                Button::new("px-sort")
                    .ghost()
                    .icon(IconName::ArrowUpDown)
                    .selected(self.proxies_sort_latency)
                    .tooltip(t!("pages.proxies.sortByLatency").to_string())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.proxies_sort_latency = !this.proxies_sort_latency;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("px-alive")
                    .ghost()
                    .icon(IconName::Eye)
                    .selected(self.proxies_alive_only)
                    .tooltip(t!("pages.proxies.aliveOnly").to_string())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.proxies_alive_only = !this.proxies_alive_only;
                        cx.notify();
                    })),
            )
            .when_some(unfix, |row, name| {
                row.child(
                    Button::new("px-unfix")
                        .ghost()
                        .icon(IconName::Refresh)
                        .tooltip(t!("tooltips.unfix").to_string())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.unfix_group(name.to_string(), cx)
                        })),
                )
            })
            .when_some(test, |row, name| {
                row.child(
                    Button::new("px-test")
                        .outline()
                        .icon(IconName::Zap)
                        .label(t!("tooltips.testLatency").to_string())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.test_group_delay(name.to_string(), cx)
                        })),
                )
            })
        };

        if groups.is_empty() {
            return v_flex()
                .size_full()
                .child(toolbar)
                .child(placeholder(
                    IconName::Globe,
                    t!("pages.proxies.noGroups").into(),
                    SharedString::default(),
                    cx,
                ))
                .into_any_element();
        }

        let selected = groups
            .iter()
            .position(|g| Some(&g.name) == focused.as_ref());
        let rows: Vec<AnyElement> = groups
            .iter()
            .enumerate()
            .map(|(i, g)| self.group_row(i, g, selected == Some(i), window, cx))
            .collect();
        let c = cx.theme().colors;
        let list = motion::highlighted(
            "proxy-groups",
            v_flex().gap(px(2.)),
            rows,
            selected,
            |d| d.rounded(px(6.)).bg(c.sel),
            window,
            cx,
        );
        let nodes = groups
            .iter()
            .find(|g| Some(&g.name) == focused.as_ref())
            .map(|g| self.render_nodes(g, cx).into_any_element())
            .unwrap_or_else(|| div().into_any_element());

        v_flex()
            .size_full()
            .child(toolbar)
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .items_start()
                    .child(
                        div()
                            .id("proxy-groups-scroll")
                            .w(px(240.))
                            .flex_none()
                            .h_full()
                            .overflow_y_scroll()
                            .border_r_1()
                            .border_color(c.border)
                            .px(px(8.))
                            .py(px(10.))
                            .child(list),
                    )
                    .child(div().flex_1().min_w_0().h_full().child(nodes)),
            )
            .into_any_element()
    }

    fn group_row(
        &self,
        ix: usize,
        group: &ProxyGroup,
        active: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = cx.theme().colors;
        let name = group.name.clone();
        let now = group.now.clone();
        let delay = group
            .all
            .iter()
            .find(|n| n.name == now)
            .and_then(|n| n.delay);
        let subtitle = if now.is_empty() {
            format!("{} {}", group.all.len(), t!("pages.proxies.nodes"))
        } else {
            match delay {
                Some(d) => format!("{now} · {d} ms"),
                None => now.to_string(),
            }
        };
        let hover = motion::hover(("group-hover", ix), window, cx);
        let bg = motion::animate(
            ("group-bg", ix),
            if hover.on && !active {
                c.row
            } else {
                c.row.opacity(0.)
            },
            window,
            cx,
        );
        hover
            .track(v_flex().id(("group", ix)))
            .gap(px(3.))
            .px(px(10.))
            .py(px(8.))
            .rounded(px(6.))
            .cursor_pointer()
            .bg(bg)
            .child(
                h_flex()
                    .gap(px(8.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_color(if active { c.text } else { c.text2 })
                            .when(active, |d| d.font_weight(FontWeight::SEMIBOLD))
                            .child(crate::ui::flags::render_line(&name)),
                    )
                    .child(chip(kind_label(group.kind.as_ref()), Tone::Muted, cx)),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(c.muted)
                    .truncate()
                    .child(crate::ui::flags::render_line(&subtitle)),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.proxies_group = Some(name.clone());
                cx.notify();
            }))
            .into_any_element()
    }

    fn render_nodes(&self, group: &ProxyGroup, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let group_name = group.name.to_string();
        let now = group.now.to_string();
        let query = self.proxies_search.read(cx).value().trim().to_lowercase();
        let alive_only = self.proxies_alive_only;
        let mut nodes: Vec<&ProxyNode> = group
            .all
            .iter()
            .filter(|n| query.is_empty() || n.name.to_lowercase().contains(&query))
            .filter(|n| !alive_only || n.delay.is_some_and(|d| d > 0))
            .collect();
        if self.proxies_sort_latency {
            nodes.sort_by_key(|n| n.delay.unwrap_or(u32::MAX));
        }
        let count = nodes.len();
        let section = nodes.into_iter().fold(Section::new(), |section, node| {
            section.row(self.node_row(group_name.clone(), node, &now, cx))
        });
        v_flex()
            .id("proxy-nodes")
            .size_full()
            .overflow_y_scroll()
            .p(px(18.))
            .gap(px(6.))
            .child(caption(
                t!("pages.proxies.nodeCount", n => count).to_string(),
                cx,
            ))
            .child(section)
    }

    fn node_row(
        &self,
        group: String,
        node: &ProxyNode,
        now: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let c = cx.theme().colors;
        let selected = node.name.as_ref() == now;
        let node_name = node.name.to_string();
        let (test_group, test_node) = (group.clone(), node_name.clone());
        h_flex()
            .id(SharedString::from(format!("node-{group}-{node_name}")))
            .h(px(38.))
            .px(px(14.))
            .gap(px(12.))
            .cursor_pointer()
            .when(selected, |r| r.bg(c.sel))
            .when(!selected, |r| r.hover(|s| s.bg(c.row)))
            .child(
                div()
                    .w(px(15.))
                    .flex_none()
                    .when(selected, |d| d.child(icon(IconName::Check, c.accent))),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .when(selected, |d| d.font_weight(FontWeight::SEMIBOLD))
                    .child(crate::ui::flags::render_line(&node_name)),
            )
            .child(
                div()
                    .flex_none()
                    .font_family(cx.theme().mono.clone())
                    .text_size(px(12.))
                    .text_color(c.muted)
                    .child(node.kind.to_lowercase()),
            )
            .child(
                div()
                    .w(px(64.))
                    .flex_none()
                    .text_right()
                    .font_family(cx.theme().mono.clone())
                    .text_size(px(12.))
                    .text_color(delay_color(node.delay, cx))
                    .child(
                        node.delay
                            .map(|d| format!("{d} ms"))
                            .unwrap_or_else(|| "—".into()),
                    ),
            )
            .child(
                Button::new(SharedString::from(format!("test-{group}-{node_name}")))
                    .ghost()
                    .size(ButtonSize::Xs)
                    .icon(IconName::Activity)
                    .tooltip(t!("tooltips.testLatency").to_string())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.test_proxy_delay(test_group.clone(), test_node.clone(), cx);
                    })),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.change_proxy(group.clone(), node_name.clone(), cx);
            }))
    }
}
