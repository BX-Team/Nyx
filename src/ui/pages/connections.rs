use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, ClipboardItem, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    SharedString, StatefulInteractiveElement, Styled, Window, div, img, px,
};
use moonglass::theme::ActiveTheme;
use moonglass::toast::ToastKind;
use moonglass::{
    Button, ButtonSize, IconName, Section, Tabs, TextField, Tone, caption, chip, h_flex, icon,
    placeholder, v_flex,
};
use rust_i18n::t;

use crate::app::state::{ConnItem, ConnProcess};
use crate::ui::pages::{body, header, updown};
use crate::ui::root::{NyxApp, fmt_bytes, nyx_dialog};

impl NyxApp {
    pub(crate) fn render_connections(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        match self.conns_detail.clone() {
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
            None => self.render_conn_list(window, cx).into_any_element(),
        }
    }

    fn render_conn_list(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let c = cx.theme().colors;
        let st = self.state.read(cx);
        let (total_up, total_down) = (st.total_up, st.total_down);
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

        let view = cx.entity().downgrade();
        let head = header(
            t!("sider.connection").to_string(),
            Some(count.to_string().into()),
            cx,
        )
        .child(
            h_flex()
                .flex_none()
                .p(px(2.))
                .rounded(px(8.))
                .border_1()
                .border_color(c.border)
                .bg(c.bg)
                .child(Tabs::new(
                    "conns-tabs",
                    vec![
                        t!("pages.connections.active").into(),
                        t!("pages.connections.closed").into(),
                    ],
                    usize::from(show_closed),
                    move |ix, _, cx| {
                        let _ = view.update(cx, |this, cx| {
                            this.conns_show_closed = ix == 1;
                            this.conns_detail = None;
                            cx.notify();
                        });
                    },
                )),
        )
        .child(updown(total_up, total_down, cx))
        .when(!show_closed && count > 0, |row| {
            row.child(
                Button::new("conns-restart")
                    .ghost()
                    .icon(IconName::Refresh)
                    .tooltip(t!("tooltips.restartConnections").to_string())
                    .on_click(cx.listener(|this, _, _, cx| this.restart_connections(cx))),
            )
            .child(
                Button::new("conns-close-all")
                    .danger()
                    .icon(IconName::Trash)
                    .label(t!("pages.connections.closeAll").to_string())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.close_all_connections(cx);
                        cx.notify();
                    })),
            )
        });

        let filter_field = TextField::new(&self.conns_filter)
            .leading(IconName::Search)
            .clearable();
        let list = if procs.is_empty() {
            placeholder(
                IconName::Network,
                t!("pages.connections.empty").into(),
                SharedString::default(),
                cx,
            )
            .into_any_element()
        } else {
            procs
                .into_iter()
                .fold(Section::new(), |section, p| {
                    section.row(self.conn_row(p, cx))
                })
                .into_any_element()
        };
        v_flex()
            .size_full()
            .child(head)
            .child(body("conns-scroll").child(filter_field).child(list))
    }

    fn conn_row(&self, p: ConnProcess, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let c = cx.theme().colors;
        let name = p.name.clone();
        // Prefer the real executable icon; fall back to the name's initials.
        let avatar = match crate::app::app_icon::for_path(p.process_path.as_ref()) {
            Some(path) => img(path)
                .size(px(28.))
                .flex_none()
                .rounded(px(6.))
                .into_any_element(),
            None => moonglass::tile(crate::ui::bar::initials(&p.name), 28., 6., cx)
                .text_color(c.accent)
                .text_size(px(11.))
                .into_any_element(),
        };
        let ids: Vec<String> = p.conns.iter().map(|c| c.id.to_string()).collect();
        h_flex()
            .id(SharedString::from(format!("conn-{name}")))
            .gap(px(12.))
            .px(px(18.))
            .h(px(48.))
            .cursor_pointer()
            .hover(|s| s.bg(c.row))
            .child(avatar)
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(8.))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(p.name.to_string()),
                    )
                    .child(chip(p.count.to_string(), Tone::Muted, cx)),
            )
            .child(updown(p.up, p.down, cx))
            .when(!self.conns_show_closed, |row| {
                row.child(
                    Button::new(SharedString::from(format!("conn-close-{}", p.name)))
                        .ghost()
                        .size(ButtonSize::Xs)
                        .icon(IconName::Close)
                        .tooltip(t!("pages.connections.close").to_string())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.close_connections(ids.clone(), cx);
                            cx.notify();
                        })),
                )
            })
            .child(icon(IconName::ChevronRight, c.muted))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.conns_detail = Some(name.clone());
                cx.notify();
            }))
    }

    fn render_conn_detail(
        &self,
        name: SharedString,
        proc: Option<ConnProcess>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let c = cx.theme().colors;
        let conns = proc.as_ref().map(|p| p.conns.clone()).unwrap_or_default();
        let (up, down) = proc.as_ref().map(|p| (p.up, p.down)).unwrap_or((0, 0));
        let count = conns.len();
        let ids: Vec<String> = conns.iter().map(|c| c.id.to_string()).collect();

        let head = h_flex()
            .flex_none()
            .gap(px(12.))
            .px(px(18.))
            .py(px(14.))
            .border_b_1()
            .border_color(c.border)
            .child(
                Button::new("conn-back")
                    .ghost()
                    .icon(IconName::ArrowLeft)
                    .tooltip(t!("common.back").to_string())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.conns_detail = None;
                        cx.notify();
                    })),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(6.))
                    .items_baseline()
                    .text_size(px(17.))
                    .child(
                        div()
                            .truncate()
                            .font_weight(FontWeight::BOLD)
                            .child(name.to_string()),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_color(c.muted)
                            .child(count.to_string()),
                    ),
            )
            .child(updown(up, down, cx))
            .when(!self.conns_show_closed && count > 0, |row| {
                row.child(
                    Button::new("conn-detail-close-all")
                        .danger()
                        .icon(IconName::Trash)
                        .label(t!("pages.connections.closeAll").to_string())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.close_connections(ids.clone(), cx);
                            cx.notify();
                        })),
                )
            });

        let list = if conns.is_empty() {
            placeholder(
                IconName::Network,
                t!("pages.connections.empty").into(),
                SharedString::default(),
                cx,
            )
            .into_any_element()
        } else {
            conns
                .into_iter()
                .enumerate()
                .fold(Section::new(), |section, (i, item)| {
                    section.row(self.conn_detail_row(i, item, cx))
                })
                .into_any_element()
        };
        v_flex()
            .size_full()
            .child(head)
            .child(body("conn-detail-scroll").child(list))
    }

    fn conn_detail_row(
        &self,
        idx: usize,
        item: ConnItem,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let c = cx.theme().colors;
        let meta = [item.chains.to_string(), item.rule.to_string()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        let popup = item.clone();
        h_flex()
            .id(("conn-row", idx))
            .gap(px(12.))
            .px(px(18.))
            .py(px(10.))
            .cursor_pointer()
            .hover(|s| s.bg(c.row))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(3.))
                    .child(
                        h_flex()
                            .gap(px(8.))
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(item.host.to_string()),
                            )
                            .when(!item.network.is_empty(), |row| {
                                row.child(chip(item.network.to_string(), Tone::Muted, cx))
                            }),
                    )
                    .when(!meta.is_empty(), |col| {
                        col.child(
                            div()
                                .truncate()
                                .text_size(px(12.))
                                .text_color(c.muted)
                                .child(meta),
                        )
                    }),
            )
            .child(updown(item.up, item.down, cx))
            .on_click(cx.listener(move |this, _, _, cx| this.open_conn_popup(popup.clone(), cx)))
    }

    fn open_conn_popup(&mut self, item: ConnItem, cx: &mut Context<Self>) {
        self.open_dialog(
            480.,
            true,
            move |this, _, cx| this.render_conn_popup(item.clone(), cx),
            cx,
        );
    }

    /// Per-connection metadata popup; every value is click-to-copy.
    fn render_conn_popup(&self, c: ConnItem, cx: &mut Context<Self>) -> AnyElement {
        let host_only = strip_port(c.host.as_ref());
        let host_frag = host_only
            .chars()
            .any(|ch| ch.is_ascii_alphabetic())
            .then(|| format!("DOMAIN-SUFFIX,{host_only}"));
        let asn_frag = c
            .dest_asn
            .split_whitespace()
            .next()
            .filter(|s| !s.is_empty())
            .map(|s| format!("IP-ASN,{s}"));
        let d = |key: &str| t!(format!("pages.connections.detail.{key}")).to_string();
        let sections = [
            (
                "routing",
                d("routing"),
                vec![
                    kv(d("rule"), c.rule.to_string(), None),
                    kv(d("chain"), c.chains.to_string(), None),
                    kv(d("type"), c.network.to_string(), None),
                ],
            ),
            (
                "network",
                d("network"),
                vec![
                    kv(d("host"), c.host.to_string(), host_frag),
                    kv(
                        d("destIp"),
                        c.dest_ip.to_string(),
                        ip_cidr_frag("IP-CIDR", c.dest_ip.as_ref()),
                    ),
                    kv(
                        d("destPort"),
                        c.dest_port.to_string(),
                        port_frag("DST-PORT", c.dest_port.as_ref()),
                    ),
                    kv(d("geoip"), c.dest_geoip.to_string(), None),
                    kv(d("asn"), c.dest_asn.to_string(), asn_frag),
                    kv(
                        d("srcIp"),
                        c.src_ip.to_string(),
                        ip_cidr_frag("SRC-IP-CIDR", c.src_ip.as_ref()),
                    ),
                    kv(
                        d("srcPort"),
                        c.src_port.to_string(),
                        port_frag("SRC-PORT", c.src_port.as_ref()),
                    ),
                ],
            ),
            (
                "process",
                d("process"),
                vec![
                    kv(
                        d("processName"),
                        c.process.to_string(),
                        (!c.process.is_empty()).then(|| format!("PROCESS-NAME,{}", c.process)),
                    ),
                    kv(d("processPath"), c.process_path.to_string(), None),
                    kv(d("dnsMode"), c.dns_mode.to_string(), None),
                ],
            ),
            (
                "traffic",
                d("traffic"),
                vec![
                    kv(d("upload"), fmt_bytes(c.up), None),
                    kv(d("download"), fmt_bytes(c.down), None),
                ],
            ),
        ];
        let mut col = v_flex()
            .id("conn-popup-scroll")
            .max_h(px(480.))
            .overflow_y_scroll()
            .gap(px(6.));
        for (key, title, rows) in sections {
            let rows: Vec<Kv> = rows
                .into_iter()
                .filter(|r| !r.value.trim().is_empty())
                .collect();
            if rows.is_empty() {
                continue;
            }
            let section = rows
                .into_iter()
                .enumerate()
                .fold(Section::new(), |section, (i, row)| {
                    section.row(kv_row(key, i, row, cx))
                });
            col = col.child(caption(title, cx)).child(section);
        }
        nyx_dialog(c.host.to_string(), col, None, cx)
    }
}

fn copy(text: String, cx: &mut gpui::App) {
    cx.write_to_clipboard(ClipboardItem::new_string(text));
    crate::app::actions::notify(ToastKind::Success, t!("common.copied").to_string(), cx);
}

fn kv_row(key: &'static str, idx: usize, row: Kv, cx: &gpui::App) -> AnyElement {
    let c = cx.theme().colors;
    let Kv { label, value, frag } = row;
    let copy_value = value.clone();
    h_flex()
        .items_start()
        .gap(px(12.))
        .px(px(14.))
        .py(px(8.))
        .child(
            div()
                .flex_none()
                .w(px(110.))
                .text_size(px(12.))
                .text_color(c.muted)
                .child(label),
        )
        .child(
            div()
                .id(SharedString::from(format!("kv-{key}-{idx}")))
                .flex_1()
                .min_w_0()
                .cursor_pointer()
                .hover(|s| s.text_color(c.accent))
                .tooltip(moonglass::tooltip(t!("common.copy").into()))
                .child(value)
                .on_click(move |_, _, cx| copy(copy_value.clone(), cx)),
        )
        .when_some(frag, |row, frag| {
            let label = frag.split(',').next().unwrap_or("").to_string();
            row.child(
                div()
                    .id(SharedString::from(format!("kvf-{key}-{idx}")))
                    .flex_none()
                    .cursor_pointer()
                    .tooltip(moonglass::tooltip(
                        t!("pages.connections.detail.copyRule").into(),
                    ))
                    .child(chip(label, Tone::Accent, cx))
                    .on_click(move |_, _, cx| copy(frag.clone(), cx)),
            )
        })
        .into_any_element()
}

/// A detail-popup row: label, value, and the rule fragment its chip copies.
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
