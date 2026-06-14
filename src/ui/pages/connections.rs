use gpui::prelude::FluentBuilder;
use gpui::{
    div, img, px, rgb, rgba, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled,
};
use gpui_component::{
    h_flex, input::Input, notification::Notification, tooltip::Tooltip, v_flex, Icon, IconName,
    StyledExt, WindowExt,
};
use rust_i18n::t;

use crate::app::state::{ConnItem, ConnProcess};
use crate::ui::root::{
    fmt_bytes, NyxApp, BLUE, CARD_BG, CARD_BORDER, GREEN, GREEN_HI, MUTED2, MUTED3, PANEL_BG, RED,
    RED_HI, SUBTLE, TEXT,
};

/// A small fixed palette for process avatars, picked by name hash.
const AVATARS: &[u32] = &[
    0xD97757, 0xE8602C, 0x5865F2, 0x37AEE2, 0x1F7A4D, 0x8B5CF6, 0x4B5563,
];

fn avatar_color(name: &str) -> u32 {
    let h = name
        .bytes()
        .fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    AVATARS[(h as usize) % AVATARS.len()]
}

impl NyxApp {
    pub(crate) fn render_connections(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.conns_detail.clone() {
            Some(name) => {
                let st = self.state.read(cx);
                let list = if self.conns_show_closed {
                    &st.closed_connections
                } else {
                    &st.connections
                };
                let proc = list.iter().find(|p| p.name == name).cloned();
                self.render_conn_detail(name, proc, cx).into_any_element()
            }
            None => self.render_conn_list(cx).into_any_element(),
        };
        div().relative().size_full().child(content).children(
            self.conn_detail_item
                .clone()
                .map(|c| self.render_conn_popup(c, cx)),
        )
    }

    /// The process list (with header totals + filter box).
    fn render_conn_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let st = self.state.read(cx);
        let total_up = st.total_up;
        let total_down = st.total_down;
        let show_closed = self.conns_show_closed;
        let filter = self.conns_filter.read(cx).value().trim().to_lowercase();
        let source = if show_closed {
            &st.closed_connections
        } else {
            &st.connections
        };
        let procs: Vec<ConnProcess> = source
            .iter()
            .filter(|p| filter.is_empty() || p.name.to_lowercase().contains(&filter))
            .cloned()
            .collect();
        let count = procs.len();

        let header = h_flex()
            .items_center()
            .justify_between()
            .px(px(22.))
            .pt(px(18.))
            .pb(px(14.))
            .child(
                div()
                    .text_xl()
                    .font_bold()
                    .text_color(rgb(TEXT))
                    .child(t!("sider.connection").to_string()),
            )
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(updown(GREEN_HI, "↑", &fmt_bytes(total_up)))
                    .child(updown(BLUE, "↓", &fmt_bytes(total_down)))
                    .when(!show_closed && count > 0, |this| {
                        this.child(
                            div()
                                .id("conns-restart")
                                .size(px(30.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(8.))
                                .bg(rgb(CARD_BG))
                                .border_1()
                                .border_color(rgb(CARD_BORDER))
                                .text_color(rgb(SUBTLE))
                                .cursor_pointer()
                                .hover(|s| s.bg(rgb(PANEL_BG)))
                                .tooltip(|window, cx| {
                                    Tooltip::new(t!("tooltips.restartConnections").to_string())
                                        .build(window, cx)
                                })
                                .child(Icon::empty().path("icons/refresh.svg").size(px(14.)))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.restart_connections(cx);
                                })),
                        )
                    })
                    .when(!show_closed && count > 0, |this| {
                        this.child(
                            div()
                                .id("conns-close-all")
                                .h(px(30.))
                                .px(px(12.))
                                .flex()
                                .items_center()
                                .gap_1p5()
                                .rounded(px(8.))
                                .bg(rgb(0x2A1614))
                                .border_1()
                                .border_color(rgb(RED))
                                .text_xs()
                                .text_color(rgb(RED_HI))
                                .cursor_pointer()
                                .hover(|s| s.bg(rgb(0x3A1E1A)))
                                .tooltip(|window, cx| {
                                    Tooltip::new(t!("pages.connections.closeAll").to_string())
                                        .build(window, cx)
                                })
                                .child(Icon::empty().path("icons/trash-2.svg").size(px(13.)))
                                .child(t!("pages.connections.closeAll").to_string())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.close_all_connections(cx);
                                    cx.notify();
                                })),
                        )
                    }),
            );

        let tab_pill = |label: String, closed: bool, on: bool, cx: &mut Context<Self>| {
            div()
                .id(SharedString::from(if closed {
                    "conns-tab-closed"
                } else {
                    "conns-tab-active"
                }))
                .px(px(12.))
                .py(px(5.))
                .rounded(px(6.))
                .text_xs()
                .cursor_pointer()
                .when(on, |t| t.bg(rgb(GREEN)).text_color(rgb(0x0B1014)))
                .when(!on, |t| t.text_color(rgb(SUBTLE)))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.conns_show_closed = closed;
                    this.conns_detail = None;
                    this.conn_detail_item = None;
                    cx.notify();
                }))
        };
        let tabs = h_flex()
            .p(px(3.))
            .gap(px(2.))
            .rounded(px(9.))
            .bg(rgb(CARD_BG))
            .border_1()
            .border_color(rgb(CARD_BORDER))
            .child(tab_pill(
                t!("pages.connections.active").to_string(),
                false,
                !show_closed,
                cx,
            ))
            .child(tab_pill(
                t!("pages.connections.closed").to_string(),
                true,
                show_closed,
                cx,
            ));

        let toolbar = h_flex()
            .items_center()
            .gap_3()
            .px(px(22.))
            .pb(px(14.))
            .child(tabs)
            .child(count_badge(count))
            .child(
                div().flex_1().child(
                    Input::new(&self.conns_filter).cleanable(true).prefix(
                        Icon::new(IconName::Search)
                            .size(px(14.))
                            .text_color(rgb(MUTED3)),
                    ),
                ),
            );

        let body = if procs.is_empty() {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_color(rgb(MUTED2))
                        .child(t!("pages.connections.empty").to_string()),
                )
                .into_any_element()
        } else {
            v_flex()
                .flex_1()
                .min_h_0()
                .px(px(22.))
                .pb(px(18.))
                .gap_2()
                .id("conns-scroll")
                .overflow_y_scroll()
                .children(procs.into_iter().map(|p| self.conn_card(p, cx)))
                .into_any_element()
        };

        v_flex()
            .size_full()
            .child(header)
            .child(toolbar)
            .child(body)
    }

    /// One clickable process card.
    fn conn_card(&self, p: ConnProcess, cx: &mut Context<Self>) -> impl IntoElement {
        let letter = p
            .name
            .chars()
            .next()
            .unwrap_or('?')
            .to_uppercase()
            .to_string();
        let name = p.name.clone();
        // Prefer the real executable icon; fall back to a colored letter tile.
        let avatar = match crate::app::app_icon::for_path(p.process_path.as_ref()) {
            Some(icon) => img(icon)
                .size(px(38.))
                .flex_none()
                .rounded(px(10.))
                .into_any_element(),
            None => div()
                .size(px(38.))
                .flex_none()
                .rounded(px(10.))
                .bg(rgb(avatar_color(p.name.as_ref())))
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(0xFFFFFF))
                .font_bold()
                .text_sm()
                .child(letter)
                .into_any_element(),
        };
        h_flex()
            .id(SharedString::from(format!("conn-{name}")))
            .items_center()
            .gap_3()
            .p(px(12.))
            .rounded(px(13.))
            .bg(rgb(CARD_BG))
            .border_1()
            .border_color(rgb(CARD_BORDER))
            .cursor_pointer()
            .hover(|s| s.border_color(rgb(0x2E3A47)))
            .child(avatar)
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(rgb(TEXT))
                                    .truncate()
                                    .child(p.name.to_string()),
                            )
                            .child(count_badge(p.count)),
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            .child(updown(GREEN_HI, "↑", &fmt_bytes(p.up)))
                            .child(updown(BLUE, "↓", &fmt_bytes(p.down))),
                    ),
            )
            .when(!self.conns_show_closed, |this| {
                let ids: Vec<String> = p.conns.iter().map(|c| c.id.to_string()).collect();
                this.child(
                    div()
                        .id(SharedString::from(format!("conn-close-{}", p.name)))
                        .size(px(28.))
                        .flex_none()
                        .rounded(px(7.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(rgb(MUTED3))
                        .cursor_pointer()
                        .hover(|s| s.text_color(rgb(RED_HI)))
                        .tooltip(|window, cx| {
                            Tooltip::new(t!("pages.connections.close").to_string())
                                .build(window, cx)
                        })
                        .child(Icon::empty().path("icons/trash-2.svg").size(px(14.)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.close_connections(ids.clone(), cx);
                            cx.notify();
                        })),
                )
            })
            .child(
                Icon::new(IconName::ChevronRight)
                    .size(px(16.))
                    .text_color(rgb(MUTED3)),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.conns_detail = Some(name.clone());
                cx.notify();
            }))
    }

    /// Detail view: the connections belonging to a single process.
    fn render_conn_detail(
        &self,
        name: SharedString,
        proc: Option<ConnProcess>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let conns = proc.as_ref().map(|p| p.conns.clone()).unwrap_or_default();
        let (up, down) = proc.as_ref().map(|p| (p.up, p.down)).unwrap_or((0, 0));
        let count = conns.len();
        let ids: Vec<String> = conns.iter().map(|c| c.id.to_string()).collect();

        let header = h_flex()
            .items_center()
            .justify_between()
            .px(px(22.))
            .pt(px(18.))
            .pb(px(14.))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .id("conn-back")
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
                                Tooltip::new(t!("common.back").to_string()).build(window, cx)
                            })
                            .child(Icon::new(IconName::ChevronLeft).size(px(16.)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.conns_detail = None;
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .text_xl()
                            .font_bold()
                            .text_color(rgb(TEXT))
                            .truncate()
                            .child(name.to_string()),
                    )
                    .child(count_badge(count)),
            )
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(updown(GREEN_HI, "↑", &fmt_bytes(up)))
                    .child(updown(BLUE, "↓", &fmt_bytes(down)))
                    .when(!self.conns_show_closed && count > 0, |this| {
                        let ids = ids.clone();
                        this.child(
                            div()
                                .id("conn-detail-close-all")
                                .h(px(30.))
                                .px(px(12.))
                                .flex()
                                .items_center()
                                .gap_1p5()
                                .rounded(px(8.))
                                .bg(rgb(0x2A1614))
                                .border_1()
                                .border_color(rgb(RED))
                                .text_xs()
                                .text_color(rgb(RED_HI))
                                .cursor_pointer()
                                .hover(|s| s.bg(rgb(0x3A1E1A)))
                                .child(Icon::empty().path("icons/trash-2.svg").size(px(13.)))
                                .child(t!("pages.connections.closeAll").to_string())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.close_connections(ids.clone(), cx);
                                    cx.notify();
                                })),
                        )
                    }),
            );

        let body = if conns.is_empty() {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_color(rgb(MUTED2))
                        .child(t!("pages.connections.empty").to_string()),
                )
                .into_any_element()
        } else {
            v_flex()
                .flex_1()
                .min_h_0()
                .px(px(22.))
                .pb(px(18.))
                .gap_2()
                .id("conn-detail-scroll")
                .overflow_y_scroll()
                .children(
                    conns
                        .into_iter()
                        .enumerate()
                        .map(|(i, c)| self.conn_detail_row(i, c, cx)),
                )
                .into_any_element()
        };

        v_flex().size_full().child(header).child(body)
    }

    /// A connection row in the detail view, clickable to open the metadata popup.
    fn conn_detail_row(&self, idx: usize, c: ConnItem, cx: &mut Context<Self>) -> impl IntoElement {
        let item = c.clone();
        conn_detail_row_inner(c)
            .id(SharedString::from(format!("conn-row-{idx}")))
            .cursor_pointer()
            .hover(|s| s.border_color(rgb(0x2E3A47)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.conn_detail_item = Some(item.clone());
                cx.notify();
            }))
    }

    /// The per-connection metadata popup. Each value is click-to-copy, with a
    /// chip that copies the matching rule fragment (`IP-CIDR,…` etc.).
    fn render_conn_popup(&self, c: ConnItem, cx: &mut Context<Self>) -> impl IntoElement {
        let host_only = strip_port(c.host.as_ref());
        let host_frag = if host_only.chars().any(|ch| ch.is_ascii_alphabetic()) {
            Some(format!("DOMAIN-SUFFIX,{host_only}"))
        } else {
            None
        };
        let asn_frag = c
            .dest_asn
            .split_whitespace()
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| format!("IP-ASN,{s}"));

        let routing = self.detail_section(
            "routing",
            t!("pages.connections.detail.routing").to_string(),
            vec![
                kv(
                    t!("pages.connections.detail.rule"),
                    c.rule.to_string(),
                    None,
                ),
                kv(
                    t!("pages.connections.detail.chain"),
                    c.chains.to_string(),
                    None,
                ),
                kv(
                    t!("pages.connections.detail.type"),
                    c.network.to_string(),
                    None,
                ),
            ],
            cx,
        );
        let network = self.detail_section(
            "network",
            t!("pages.connections.detail.network").to_string(),
            vec![
                kv(
                    t!("pages.connections.detail.host"),
                    c.host.to_string(),
                    host_frag,
                ),
                kv(
                    t!("pages.connections.detail.destIp"),
                    c.dest_ip.to_string(),
                    ip_cidr_frag("IP-CIDR", c.dest_ip.as_ref()),
                ),
                kv(
                    t!("pages.connections.detail.destPort"),
                    c.dest_port.to_string(),
                    port_frag("DST-PORT", c.dest_port.as_ref()),
                ),
                kv(
                    t!("pages.connections.detail.geoip"),
                    c.dest_geoip.to_string(),
                    None,
                ),
                kv(
                    t!("pages.connections.detail.asn"),
                    c.dest_asn.to_string(),
                    asn_frag,
                ),
                kv(
                    t!("pages.connections.detail.srcIp"),
                    c.src_ip.to_string(),
                    ip_cidr_frag("SRC-IP-CIDR", c.src_ip.as_ref()),
                ),
                kv(
                    t!("pages.connections.detail.srcPort"),
                    c.src_port.to_string(),
                    port_frag("SRC-PORT", c.src_port.as_ref()),
                ),
            ],
            cx,
        );
        let process = self.detail_section(
            "process",
            t!("pages.connections.detail.process").to_string(),
            vec![
                kv(
                    t!("pages.connections.detail.processName"),
                    c.process.to_string(),
                    (!c.process.is_empty()).then(|| format!("PROCESS-NAME,{}", c.process)),
                ),
                kv(
                    t!("pages.connections.detail.processPath"),
                    c.process_path.to_string(),
                    None,
                ),
                kv(
                    t!("pages.connections.detail.dnsMode"),
                    c.dns_mode.to_string(),
                    None,
                ),
            ],
            cx,
        );
        let traffic = self.detail_section(
            "traffic",
            t!("pages.connections.detail.traffic").to_string(),
            vec![
                kv(t!("pages.connections.detail.upload"), fmt_bytes(c.up), None),
                kv(
                    t!("pages.connections.detail.download"),
                    fmt_bytes(c.down),
                    None,
                ),
            ],
            cx,
        );

        div()
            .id("conn-popup-scrim")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x000000B0))
            // Block all mouse interaction with the list behind the modal.
            .occlude()
            .on_click(cx.listener(|this, _, _, cx| {
                this.conn_detail_item = None;
                cx.notify();
            }))
            .child(
                v_flex()
                    .w(px(460.))
                    .max_h(px(560.))
                    .rounded_xl()
                    .border_1()
                    .border_color(rgb(CARD_BORDER))
                    .bg(rgb(CARD_BG))
                    .p_4()
                    .gap_3()
                    // Swallow clicks inside the card so they don't dismiss it.
                    .id("conn-popup-card")
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
                                    .child(c.host.to_string()),
                            )
                            .child(
                                div()
                                    .id("conn-popup-close")
                                    .size(px(28.))
                                    .rounded(px(7.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(rgb(CARD_BG))
                                    .border_1()
                                    .border_color(rgb(CARD_BORDER))
                                    .text_color(rgb(SUBTLE))
                                    .cursor_pointer()
                                    .child(Icon::new(IconName::Close).size(px(14.)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.conn_detail_item = None;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .min_h_0()
                            .id("conn-popup-scroll")
                            .overflow_y_scroll()
                            .gap_3()
                            .child(routing)
                            .child(network)
                            .child(process)
                            .child(traffic),
                    ),
            )
    }

    /// A titled card of detail rows; empty-value rows (and all-empty sections) are dropped.
    fn detail_section(
        &self,
        key: &'static str,
        title: String,
        rows: Vec<Kv>,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let rows: Vec<Kv> = rows
            .into_iter()
            .filter(|r| !r.value.trim().is_empty())
            .collect();
        if rows.is_empty() {
            return div().into_any_element();
        }
        v_flex()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .font_semibold()
                    .text_color(rgb(MUTED3))
                    .child(title.to_uppercase()),
            )
            .child(
                v_flex()
                    .rounded(px(10.))
                    .bg(rgb(PANEL_BG))
                    .border_1()
                    .border_color(rgb(CARD_BORDER))
                    .px(px(12.))
                    .children(
                        rows.into_iter()
                            .enumerate()
                            .map(|(i, r)| self.kv_row(key, i, r, cx)),
                    ),
            )
            .into_any_element()
    }

    /// One `label : value` row; click-to-copy with an optional rule-fragment chip.
    fn kv_row(
        &self,
        key: &'static str,
        idx: usize,
        row: Kv,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let Kv { label, value, frag } = row;
        let copy_value = value.clone();
        let value_id = SharedString::from(format!("kv-{key}-{idx}"));
        let frag_id = SharedString::from(format!("kvf-{key}-{idx}"));

        h_flex()
            .items_start()
            .gap_3()
            .py(px(6.))
            .child(
                div()
                    .flex_none()
                    .w(px(108.))
                    .text_xs()
                    .text_color(rgb(MUTED3))
                    .child(label),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .items_start()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .id(value_id)
                            .flex_1()
                            .min_w_0()
                            .text_xs()
                            .text_color(rgb(TEXT))
                            .cursor_pointer()
                            .hover(|s| s.text_color(rgb(GREEN_HI)))
                            .tooltip(|window, cx| {
                                Tooltip::new(t!("common.copy").to_string()).build(window, cx)
                            })
                            .child(value)
                            .on_click(cx.listener(move |_this, _, window, cx| {
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                    copy_value.clone(),
                                ));
                                window
                                    .push_notification(Notification::info(t!("common.copied")), cx);
                            })),
                    )
                    .when_some(frag, |this, frag| {
                        let chip = frag.split(',').next().unwrap_or("").to_string();
                        this.child(
                            div()
                                .id(frag_id)
                                .flex_none()
                                .px(px(6.))
                                .py(px(1.))
                                .rounded(px(5.))
                                .bg(rgb(0x1A2530))
                                .text_color(rgb(SUBTLE))
                                .text_xs()
                                .cursor_pointer()
                                .hover(|s| s.text_color(rgb(GREEN_HI)))
                                .tooltip(|window, cx| {
                                    Tooltip::new(
                                        t!("pages.connections.detail.copyRule").to_string(),
                                    )
                                    .build(window, cx)
                                })
                                .child(chip)
                                .on_click(cx.listener(move |_this, _, window, cx| {
                                    cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                        frag.clone(),
                                    ));
                                    window.push_notification(
                                        Notification::info(t!("common.copied")),
                                        cx,
                                    );
                                })),
                        )
                    }),
            )
    }
}

/// The visual body of a connection row (no interactivity).
fn conn_detail_row_inner(c: ConnItem) -> gpui::Div {
    v_flex()
        .gap_1p5()
        .p(px(12.))
        .rounded(px(12.))
        .bg(rgb(CARD_BG))
        .border_1()
        .border_color(rgb(CARD_BORDER))
        .child(
            h_flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_sm()
                        .font_semibold()
                        .text_color(rgb(TEXT))
                        .truncate()
                        .child(c.host.to_string()),
                )
                .when(!c.network.is_empty(), |this| {
                    this.child(
                        div()
                            .flex_none()
                            .px(px(6.))
                            .py(px(1.))
                            .rounded(px(5.))
                            .bg(rgb(0x1A2530))
                            .text_color(rgb(SUBTLE))
                            .text_xs()
                            .child(c.network.to_string()),
                    )
                }),
        )
        .when(!c.chains.is_empty(), |this| {
            this.child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED2))
                    .truncate()
                    .child(c.chains.to_string()),
            )
        })
        .child(
            h_flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_xs()
                        .text_color(rgb(MUTED3))
                        .truncate()
                        .child(c.rule.to_string()),
                )
                .child(
                    h_flex()
                        .gap_3()
                        .flex_none()
                        .child(updown(GREEN_HI, "↑", &fmt_bytes(c.up)))
                        .child(updown(BLUE, "↓", &fmt_bytes(c.down))),
                ),
        )
}

/// A green pill showing a connection count.
fn count_badge(n: usize) -> impl IntoElement {
    div()
        .px(px(6.))
        .py(px(1.))
        .rounded(px(50.))
        .bg(rgb(0x1F7A4D))
        .text_color(rgb(0xD6F7E6))
        .text_xs()
        .child(n.to_string())
}

/// An up/down arrow + value pair (mono-ish, colored).
fn updown(color: u32, arrow: &str, value: &str) -> impl IntoElement {
    let s: SharedString = format!("{arrow} {value}").into();
    div().text_xs().text_color(rgb(color)).child(s)
}

/// A row spec for the detail popup: label, value, and an optional rule fragment
/// (the chip that copies e.g. `IP-CIDR,1.2.3.4/32`).
struct Kv {
    label: String,
    value: String,
    frag: Option<String>,
}

fn kv(label: impl Into<String>, value: String, frag: Option<String>) -> Kv {
    Kv {
        label: label.into(),
        value,
        frag,
    }
}

/// Strips a trailing `:port` from a host (leaves bare IPv6 addresses untouched).
fn strip_port(host: &str) -> &str {
    if let Some(idx) = host.rfind(':') {
        let after_digits = host[idx + 1..].chars().all(|c| c.is_ascii_digit());
        let single_colon = host[..idx].rfind(':').is_none();
        if idx > 0 && after_digits && single_colon {
            return &host[..idx];
        }
    }
    host
}

/// `IP-CIDR,<ip>/32` (or `/128` for IPv6); `None` for an empty value.
fn ip_cidr_frag(prefix: &str, ip: &str) -> Option<String> {
    let ip = ip.trim();
    if ip.is_empty() {
        return None;
    }
    let suffix = if ip.contains(':') { "/128" } else { "/32" };
    Some(format!("{prefix},{ip}{suffix}"))
}

/// `<prefix>,<port>`; `None` for an empty / zero port.
fn port_frag(prefix: &str, port: &str) -> Option<String> {
    let port = port.trim();
    if port.is_empty() || port == "0" {
        return None;
    }
    Some(format!("{prefix},{port}"))
}
