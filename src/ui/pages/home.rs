use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, px, relative,
};
use moonglass::theme::ActiveTheme;
use moonglass::{
    Button, ButtonSize, IconName, Section, Tone, W_SEMIBOLD, caption, h_flex, icon, meter,
    placeholder, v_flex,
};
use rust_i18n::t;
use serde_json::Value;

use crate::ui::pages::{body, header, updown};
use crate::ui::root::{NyxApp, Route, delay_color, fmt_bytes, fmt_speed};

struct SubStats {
    has_traffic: bool,
    used: u64,
    total: u64,
    remaining: u64,
    days: Option<i64>,
    expire_label: Option<String>,
    announce: Vec<String>,
}

fn sub_stats(item: &Option<Value>) -> SubStats {
    let extra = item.as_ref().and_then(|i| i.get("extra"));
    let upload = extra
        .and_then(|e| e.get("upload"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let download = extra
        .and_then(|e| e.get("download"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total = extra
        .and_then(|e| e.get("total"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let expire = extra
        .and_then(|e| e.get("expire"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let used = upload.saturating_add(download);
    let remaining = total.saturating_sub(used);

    let (days, expire_label) = if expire > 0 {
        let now = chrono::Utc::now().timestamp();
        let days = ((expire - now) / 86_400).max(0);
        let label =
            chrono::DateTime::from_timestamp(expire, 0).map(|d| d.format("%Y-%m-%d").to_string());
        (Some(days), label)
    } else {
        (None, None)
    };

    let announce = item
        .as_ref()
        .and_then(|i| i.get("announce"))
        .and_then(Value::as_str)
        .map(|s| {
            s.lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    SubStats {
        has_traffic: total > 0 || used > 0,
        used,
        total,
        remaining,
        days,
        expire_label,
        announce,
    }
}

/// The profile's `supportUrl` plus whether it is a Telegram link, for the icon.
fn support_link(item: &Option<Value>) -> Option<(String, bool)> {
    let url = item
        .as_ref()
        .and_then(|i| i.get("supportUrl"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())?
        .to_string();
    let lower = url.to_lowercase();
    let is_telegram =
        lower.starts_with("tg:") || lower.contains("t.me") || lower.contains("telegram");
    Some((url, is_telegram))
}

impl NyxApp {
    pub(crate) fn render_home(&self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let st = self.state.read(cx);
        if st.profiles.is_empty() {
            return self.render_home_empty(cx);
        }
        let c = cx.theme().colors;
        let mono = cx.theme().mono.clone();
        let profile_name = st.current_profile_name.clone();
        let current = st.groups.iter().find(|g| !g.now.is_empty()).map(|g| {
            let delay = g.all.iter().find(|n| n.name == g.now).and_then(|n| n.delay);
            (
                g.name.to_string(),
                g.kind.to_string(),
                g.now.to_string(),
                delay,
            )
        });
        let (total_up, total_down) = (st.total_up, st.total_down);
        let (up_speed, down_speed) = (st.up_speed, st.down_speed);
        let history: Vec<u64> = st.history.iter().map(|(u, d)| u + d).collect();
        let stats = sub_stats(&st.current_profile_item);
        let support = support_link(&st.current_profile_item);
        let failure = st
            .core_status
            .failed()
            .map(|(kind, detail)| (crate::app::state::failure_key(kind), detail.clone()));

        let support_btn = support.map(|(url, is_telegram)| {
            Button::new("home-support")
                .ghost()
                .icon(if is_telegram {
                    IconName::Custom("icons/telegram.svg")
                } else {
                    IconName::ExternalLink
                })
                .tooltip(if is_telegram {
                    t!("tooltips.telegram").to_string()
                } else {
                    t!("tooltips.support").to_string()
                })
                .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url)))
        });
        let head = header(t!("sider.home").to_string(), profile_name, cx)
            .children(support_btn)
            .child(
                Button::new("home-refresh")
                    .ghost()
                    .icon(IconName::Refresh)
                    .tooltip(t!("tooltips.refresh").to_string())
                    .on_click(cx.listener(|this, _, _, cx| this.refresh_subscription(cx))),
            );

        let failure = failure.map(|(key, detail)| {
            Section::new().row(
                h_flex()
                    .gap(px(12.))
                    .px(px(18.))
                    .py(px(14.))
                    .items_start()
                    .child(icon(IconName::Alert, moonglass::theme::danger()).mt(px(2.)))
                    .child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(4.))
                            .child(
                                div()
                                    .font_weight(W_SEMIBOLD)
                                    .text_color(moonglass::theme::danger())
                                    .child(t!(key).to_string()),
                            )
                            .child(div().text_color(c.muted).child(detail)),
                    ),
            )
        });

        let node_row = {
            let (group, kind, node, delay) = current.unwrap_or_default();
            h_flex()
                .id("home-node")
                .gap(px(16.))
                .px(px(18.))
                .py(px(14.))
                .cursor_pointer()
                .hover(|s| s.bg(c.row))
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .font_weight(W_SEMIBOLD)
                                .text_size(px(14.))
                                .child(t!("pages.home.currentNode").to_string()),
                        )
                        .child(div().mt(px(4.)).text_color(c.muted).truncate().child(
                            if group.is_empty() {
                                "—".to_string()
                            } else {
                                format!("{group} · {kind}")
                            },
                        )),
                )
                .child(div().max_w(px(320.)).truncate().text_color(c.text2).child(
                    crate::ui::flags::render_line(if node.is_empty() { "—" } else { &node }),
                ))
                .child(
                    div()
                        .w(px(64.))
                        .flex_none()
                        .text_right()
                        .font_family(mono.clone())
                        .text_size(px(12.))
                        .text_color(delay_color(delay, cx))
                        .child(delay.map(|d| format!("{d} ms")).unwrap_or_default()),
                )
                .child(icon(IconName::ChevronRight, c.muted))
                .on_click(
                    cx.listener(|this, _, window, cx| this.navigate(Route::Proxies, window, cx)),
                )
        };

        let peak = history.iter().copied().max().unwrap_or(0).max(1);
        let bars = h_flex()
            .h(px(64.))
            .w_full()
            .items_end()
            .gap(px(2.))
            .children(history.iter().map(|v| {
                let f = (*v as f32 / peak as f32).max(0.03);
                div()
                    .flex_1()
                    .h(relative(f))
                    .rounded_t(px(2.))
                    .bg(c.accent.opacity(0.35 + 0.65 * f))
            }));
        let traffic = Section::new().row(
            v_flex()
                .gap(px(10.))
                .px(px(18.))
                .py(px(14.))
                .child(
                    h_flex()
                        .gap(px(12.))
                        .child(
                            div()
                                .flex_1()
                                .font_weight(W_SEMIBOLD)
                                .text_size(px(14.))
                                .child(t!("pages.home.traffic").to_string()),
                        )
                        .child(
                            div()
                                .font_family(mono.clone())
                                .text_size(px(12.))
                                .text_color(c.muted)
                                .child(format!(
                                    "↓ {} ↑ {}",
                                    fmt_speed(down_speed),
                                    fmt_speed(up_speed)
                                )),
                        ),
                )
                .child(bars)
                .child(
                    h_flex()
                        .child(
                            div()
                                .flex_1()
                                .text_color(c.muted)
                                .child(t!("pages.home.sessionTotal").to_string()),
                        )
                        .child(updown(total_up, total_down, cx)),
                ),
        );

        let subscription = stats.has_traffic.then(|| {
            let fraction = if stats.total > 0 {
                stats.used as f32 / stats.total as f32
            } else {
                0.
            };
            let tone = if fraction >= 0.9 {
                Tone::Warn
            } else {
                Tone::Accent
            };
            let expiry = match (&stats.expire_label, stats.days) {
                (Some(date), Some(days)) => {
                    t!("pages.home.expiresIn", date => date, n => days).to_string()
                }
                _ => t!("pages.home.never").to_string(),
            };
            let remaining = if stats.total > 0 {
                fmt_bytes(stats.remaining)
            } else {
                "∞".to_string()
            };
            Section::new().row(
                v_flex()
                    .gap(px(10.))
                    .px(px(18.))
                    .py(px(14.))
                    .child(
                        h_flex()
                            .gap(px(12.))
                            .child(
                                div()
                                    .flex_1()
                                    .font_weight(W_SEMIBOLD)
                                    .text_size(px(14.))
                                    .child(t!("pages.home.subscription").to_string()),
                            )
                            .child(
                                div()
                                    .font_family(mono.clone())
                                    .text_size(px(12.))
                                    .text_color(c.muted)
                                    .child(expiry),
                            ),
                    )
                    .when(stats.total > 0, |col| col.child(meter(fraction, tone, cx)))
                    .child(
                        h_flex()
                            .font_family(mono.clone())
                            .text_size(px(12.))
                            .text_color(c.muted)
                            .child(div().flex_1().child(if stats.total > 0 {
                                format!("{} / {}", fmt_bytes(stats.used), fmt_bytes(stats.total))
                            } else {
                                fmt_bytes(stats.used)
                            }))
                            .child(t!("pages.home.remainingValue", value => remaining).to_string()),
                    ),
            )
        });

        let news = (!stats.announce.is_empty()).then(|| {
            stats.announce.iter().fold(Section::new(), |section, line| {
                section.row(
                    div()
                        .px(px(18.))
                        .py(px(10.))
                        .text_color(c.text2)
                        .child(line.clone()),
                )
            })
        });

        v_flex()
            .size_full()
            .child(head)
            .child(
                body("home-scroll")
                    .children(failure)
                    .child(Section::new().row(node_row))
                    .child(traffic)
                    .children(subscription)
                    .when_some(news, |col, news| {
                        col.child(caption(t!("pages.home.subscriptionNews").to_string(), cx))
                            .child(news)
                    }),
            )
            .into_any_element()
    }

    fn render_home_empty(&self, cx: &mut Context<Self>) -> AnyElement {
        placeholder(
            IconName::Folder,
            t!("pages.home.emptyTitle").into(),
            t!("pages.home.emptyBody").into(),
            cx,
        )
        .child(
            Button::new("home-empty-add")
                .primary()
                .size(ButtonSize::Md)
                .icon(IconName::Plus)
                .label(t!("pages.home.emptyCta").to_string())
                .mt(px(8.))
                .on_click(cx.listener(|this, _, window, cx| this.open_profile_add(window, cx))),
        )
        .into_any_element()
    }
}
