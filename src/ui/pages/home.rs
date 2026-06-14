use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, rgb, rgba, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex, v_flex, Icon, IconName, Sizable, StyledExt,
};
use rust_i18n::t;
use serde_json::Value;

use crate::ui::root::{
    fmt_bytes, power_on_bg, NyxApp, CARD_BG, CARD_BORDER, GOOD, GREEN, MUTED, PANEL_BG, STROKE,
    TEXT,
};

/// Parsed subscription stats from a profile's `extra` + `announce`.
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

/// The profile's `supportUrl`, if any, plus whether it points at Telegram
/// (`tg:` scheme or a `t.me` / `telegram` host) — used to pick the button icon.
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

fn fmt_elapsed(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

impl NyxApp {
    pub(crate) fn render_home(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let st = self.state.read(cx);
        let tun = st.tun_enabled;
        let total_up = st.total_up;
        let total_down = st.total_down;
        let profile_name = st
            .current_profile_name
            .clone()
            .unwrap_or_else(|| SharedString::from("—"));
        let current_proxy = st
            .groups
            .iter()
            .find(|g| !g.now.is_empty())
            .map(|g| (g.now.to_string(), g.kind.to_string()));
        let stats = sub_stats(&st.current_profile_item);
        let stats_open = self.stats_open;

        let elapsed = self
            .connected_since
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0);
        let status = if tun {
            t!("pages.home.connected").to_string()
        } else {
            t!("pages.home.disconnected").to_string()
        };

        let support = support_link(&st.current_profile_item);

        let main = v_flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .child(self.render_topbar(&profile_name, tun, &status, support, cx))
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .items_center()
                    .justify_center()
                    .gap_4()
                    .child(
                        div()
                            .text_xl()
                            .font_bold()
                            .text_color(rgb(TEXT))
                            .child(status.to_uppercase()),
                    )
                    .child(self.render_power_button(tun, cx))
                    .when(tun, |this| {
                        this.child(
                            div()
                                .text_lg()
                                .font_bold()
                                .text_color(rgb(TEXT))
                                .child(fmt_elapsed(elapsed)),
                        )
                        .child(render_speeds(total_up, total_down))
                    }),
            )
            .child(render_proxy_card(current_proxy, cx));

        h_flex().size_full().p_5().gap_4().child(main).when(
            stats_open && (stats.has_traffic || !stats.announce.is_empty()),
            |this| this.child(render_stats(&stats)),
        )
    }

    fn render_topbar(
        &self,
        profile: &str,
        tun: bool,
        status: &str,
        support: Option<(String, bool)>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let dot = if tun { GOOD } else { MUTED };
        let support_btn = support.map(|(url, is_telegram)| {
            let icon = if is_telegram {
                Icon::empty().path("icons/telegram.svg")
            } else {
                Icon::new(IconName::Globe)
            };
            let tip = if is_telegram {
                t!("tooltips.telegram")
            } else {
                t!("tooltips.support")
            };
            Button::new("home-support")
                .ghost()
                .small()
                .icon(icon)
                .tooltip(tip.to_string())
                .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url)))
        });
        h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .pb_2()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .font_bold()
                            .text_color(rgb(TEXT))
                            .child(profile.to_string()),
                    )
                    .child(div().size_2().rounded_full().bg(rgb(dot)))
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(MUTED))
                            .child(status.to_string()),
                    ),
            )
            .child(
                h_flex()
                    .gap_1()
                    .children(support_btn)
                    .child(
                        Button::new("home-refresh")
                            .ghost()
                            .small()
                            .icon(Icon::empty().path("icons/refresh.svg"))
                            .tooltip(t!("tooltips.refresh").to_string())
                            .on_click(cx.listener(|this, _, _, cx| this.refresh_subscription(cx))),
                    )
                    .child(
                        Button::new("home-stats-toggle")
                            .ghost()
                            .small()
                            .icon(IconName::ChevronRight)
                            .tooltip(t!("tooltips.toggleStats").to_string())
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_stats(cx))),
                    ),
            )
    }

    fn render_power_button(&self, tun: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let icon_color = if tun { 0x06140C } else { MUTED };
        let inner = if tun {
            div().size(px(116.)).rounded_full().bg(power_on_bg())
        } else {
            div()
                .size(px(116.))
                .rounded_full()
                .bg(rgb(CARD_BG))
                .border_1()
                .border_color(rgb(CARD_BORDER))
        };
        let icon = if tun { IconName::Pause } else { IconName::Play };
        div()
            .id("power-button")
            .size(px(116.))
            .rounded_full()
            .cursor_pointer()
            .child(
                inner
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(Icon::new(icon).large().text_color(rgb(icon_color))),
            )
            .on_click(cx.listener(|this, _, _, cx| this.toggle_tun(cx)))
    }
}

fn render_speeds(up: u64, down: u64) -> impl IntoElement {
    h_flex()
        .gap_4()
        .items_center()
        .child(
            h_flex()
                .gap_1p5()
                .items_center()
                .child(Icon::new(IconName::ArrowUp).small().text_color(rgb(GREEN)))
                .child(div().text_sm().text_color(rgb(MUTED)).child(fmt_bytes(up))),
        )
        .child(div().w(px(1.)).h(px(12.)).bg(rgba(STROKE)))
        .child(
            h_flex()
                .gap_1p5()
                .items_center()
                .child(
                    Icon::new(IconName::ArrowDown)
                        .small()
                        .text_color(rgb(GREEN)),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(MUTED))
                        .child(fmt_bytes(down)),
                ),
        )
}

fn render_proxy_card(
    current: Option<(String, String)>,
    cx: &mut Context<NyxApp>,
) -> impl IntoElement {
    let (name, kind) = current.unwrap_or_else(|| ("—".to_string(), String::new()));
    div().flex().justify_center().child(
        div()
            .id("home-proxy-card")
            .w(px(320.))
            .rounded_xl()
            .border_1()
            .border_color(rgba(STROKE))
            .bg(rgb(CARD_BG))
            .cursor_pointer()
            .pl_3()
            .pr_2()
            .py_2()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        v_flex()
                            .min_w_0()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(rgb(TEXT))
                                    .truncate()
                                    .child(crate::ui::flags::render_name(&name)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(MUTED))
                                    .truncate()
                                    .child(kind.to_uppercase()),
                            ),
                    )
                    .child(Icon::new(IconName::ChevronRight).text_color(rgb(MUTED))),
            )
            .on_click(cx.listener(|this, _, _, cx| {
                this.route = crate::ui::root::Route::Proxies;
                cx.notify();
            })),
    )
}

fn section_header(text: &str) -> impl IntoElement {
    div()
        .text_xs()
        .font_semibold()
        .text_color(rgb(MUTED))
        .child(text.to_uppercase())
}

fn stat_tile() -> gpui::Div {
    v_flex()
        .gap_1()
        .rounded_lg()
        .border_1()
        .border_color(rgba(STROKE))
        .bg(rgb(PANEL_BG))
        .p_3()
}

fn render_stats(stats: &SubStats) -> impl IntoElement {
    let mut col = v_flex().gap_3();

    if stats.has_traffic {
        let remaining = if stats.total > 0 {
            fmt_bytes(stats.remaining)
        } else {
            "∞".to_string()
        };
        let used_total = if stats.total > 0 {
            format!("{} / {}", fmt_bytes(stats.used), fmt_bytes(stats.total))
        } else {
            String::new()
        };
        let days = stats
            .days
            .map(|d| d.to_string())
            .unwrap_or_else(|| "∞".to_string());
        let expire = stats
            .expire_label
            .clone()
            .unwrap_or_else(|| t!("pages.home.never").to_string());

        col = col
            .child(section_header(&t!("pages.home.statistics")))
            .child(
                stat_tile()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED))
                            .child(t!("pages.home.trafficRemaining").to_string()),
                    )
                    .child(
                        div()
                            .text_xl()
                            .font_bold()
                            .text_color(rgb(TEXT))
                            .child(remaining),
                    )
                    .when(!used_total.is_empty(), |this| {
                        this.child(div().text_xs().text_color(rgb(MUTED)).child(used_total))
                    }),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        stat_tile()
                            .flex_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(MUTED))
                                    .child(t!("pages.home.daysRemaining").to_string()),
                            )
                            .child(
                                div()
                                    .text_base()
                                    .font_bold()
                                    .text_color(rgb(TEXT))
                                    .child(days),
                            ),
                    )
                    .child(
                        stat_tile()
                            .flex_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(MUTED))
                                    .child(t!("pages.home.expires").to_string()),
                            )
                            .child(
                                div()
                                    .text_base()
                                    .font_bold()
                                    .text_color(rgb(TEXT))
                                    .truncate()
                                    .child(expire),
                            ),
                    ),
            );
    }

    if !stats.announce.is_empty() {
        col = col
            .child(section_header(&t!("pages.home.subscriptionNews")))
            .child(
                v_flex()
                    .gap_1p5()
                    .children(stats.announce.iter().map(|line| {
                        h_flex()
                            .w_full()
                            .gap_2()
                            .items_start()
                            .child(div().flex_shrink_0().text_color(rgb(MUTED)).child("·"))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_sm()
                                    .text_color(rgb(TEXT))
                                    .child(line.clone()),
                            )
                    })),
            );
    }

    v_flex()
        .w(px(280.))
        .h_full()
        .rounded_xl()
        .border_1()
        .border_color(rgba(STROKE))
        .bg(rgb(CARD_BG))
        .p_3()
        .child(col)
}
