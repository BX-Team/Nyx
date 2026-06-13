use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, rgb, rgba, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled,
};
use gpui_component::{h_flex, v_flex, Icon, StyledExt};
use rust_i18n::t;

use crate::app::state::LogLine;
use crate::ui::root::{
    LogFilter, NyxApp, AMBER, BLUE, CARD_BG, CARD_BORDER, GREEN, MUTED2, MUTED4, PANEL_BG, RED,
    SUBTLE, TEXT,
};

/// Visual treatment for a log level: chip text, chip colour, message colour.
fn level_style(level: &str) -> (&'static str, u32, u32) {
    match level {
        "warning" | "warn" => ("WARN", AMBER, 0xE6C98A),
        "error" | "err" => ("ERR", RED, 0xEE9D95),
        "debug" | "dbg" => ("DBG", BLUE, MUTED2),
        _ => ("INFO", GREEN, 0xCDD8E3),
    }
}

fn matches(filter: LogFilter, level: &str) -> bool {
    match filter {
        LogFilter::All => true,
        LogFilter::Info => level == "info",
        LogFilter::Warning => level == "warning" || level == "warn",
        LogFilter::Error => level == "error" || level == "err",
    }
}

impl NyxApp {
    pub(crate) fn render_logs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let filter = self.logs_filter;
        let st = self.state.read(cx);
        // Cap rendered rows (no virtualization) — newest 400 after filtering.
        let lines: Vec<LogLine> = st
            .logs
            .iter()
            .filter(|l| matches(filter, l.level.as_ref()))
            .rev()
            .take(400)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        // Stick to the bottom whenever new log lines have arrived since the last
        // render, so the latest output is always visible without manual
        // scrolling. Keyed off the monotonic `log_seq` (not `logs.len()`, which
        // saturates at the ring-buffer cap once full).
        let seq = st.log_seq;
        if self.logs_seen.get() != seq {
            self.logs_seen.set(seq);
            self.logs_scroll.scroll_to_bottom();
        }

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
                    .child(t!("sider.logs").to_string()),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(self.logs_segmented(cx))
                    .child(
                        h_flex()
                            .id("logs-clear")
                            .h(px(32.))
                            .px(px(12.))
                            .gap_2()
                            .items_center()
                            .rounded(px(8.))
                            .bg(rgb(CARD_BG))
                            .border_1()
                            .border_color(rgb(CARD_BORDER))
                            .text_color(rgb(SUBTLE))
                            .cursor_pointer()
                            .child(Icon::empty().path("icons/trash-2.svg").size(px(14.)))
                            .child(div().text_xs().child(t!("pages.logs.clear").to_string()))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.update(cx, |s, c| s.clear_logs(c));
                            })),
                    ),
            );

        let console = v_flex()
            .flex_1()
            .min_h_0()
            .mx(px(14.))
            .mb(px(16.))
            .rounded(px(12.))
            .bg(rgb(PANEL_BG))
            .border_1()
            .border_color(rgb(CARD_BORDER))
            .id("logs-scroll")
            .overflow_y_scroll()
            .track_scroll(&self.logs_scroll)
            .children(
                lines
                    .into_iter()
                    .enumerate()
                    .map(|(i, l)| log_row(l, i % 2 == 1)),
            );

        v_flex().size_full().child(header).child(console)
    }

    /// The Все / Info / Warn / Error level filter.
    fn logs_segmented(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let cur = self.logs_filter;
        let pill = |label: String, f: LogFilter, cx: &mut Context<Self>| {
            div()
                .id(SharedString::from(format!("logf-{label}")))
                .px(px(11.))
                .py(px(5.))
                .rounded(px(6.))
                .text_xs()
                .cursor_pointer()
                .when(cur == f, |this| {
                    this.bg(rgb(GREEN)).text_color(rgb(0x0B1014))
                })
                .when(cur != f, |this| this.text_color(rgb(SUBTLE)))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.logs_filter = f;
                    cx.notify();
                }))
        };
        h_flex()
            .p(px(3.))
            .gap(px(2.))
            .rounded(px(9.))
            .bg(rgb(CARD_BG))
            .border_1()
            .border_color(rgb(CARD_BORDER))
            .child(pill(t!("pages.logs.all").to_string(), LogFilter::All, cx))
            .child(pill("Info".to_string(), LogFilter::Info, cx))
            .child(pill("Warn".to_string(), LogFilter::Warning, cx))
            .child(pill("Error".to_string(), LogFilter::Error, cx))
    }
}

fn log_row(l: LogLine, striped: bool) -> impl IntoElement {
    let (chip, chip_bg, msg_color) = level_style(l.level.as_ref());
    h_flex()
        .gap_3()
        .px(px(16.))
        .py(px(8.))
        .items_start()
        .when(striped, |this| this.bg(rgba(0xFFFFFF04)))
        .child(
            div()
                .flex_none()
                .text_xs()
                .text_color(rgb(MUTED4))
                .child(l.time.clone()),
        )
        .child(
            div()
                .flex_none()
                .px(px(6.))
                .rounded(px(4.))
                .bg(rgb(chip_bg))
                .text_color(rgb(0x0B1014))
                .text_xs()
                .child(chip),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_xs()
                .text_color(rgb(msg_color))
                .child(crate::ui::flags::render_name(l.message.as_ref())),
        )
}
