use gpui::{
    App, Context, Hsla, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement,
    Styled, div, px,
};
use moonglass::theme::ActiveTheme;
use moonglass::{Button, IconName, Tabs, h_flex, v_flex};
use rust_i18n::t;

use crate::app::state::LogLine;
use crate::ui::pages::header;
use crate::ui::root::{LogFilter, NyxApp};

const FILTERS: [LogFilter; 4] = [
    LogFilter::All,
    LogFilter::Info,
    LogFilter::Warning,
    LogFilter::Error,
];

fn level_style(level: &str, cx: &App) -> (&'static str, Hsla) {
    let c = cx.theme().colors;
    match level {
        "warning" | "warn" => ("WARN", c.warn),
        "error" | "err" => ("ERR", moonglass::theme::danger()),
        "debug" | "dbg" => ("DBG", c.muted),
        _ => ("INFO", c.accent),
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
    pub(crate) fn render_logs(
        &self,
        _window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let c = cx.theme().colors;
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

        // Autoscroll keys off monotonic `log_seq`, since `logs.len()` saturates at the cap.
        let seq = st.log_seq;
        if self.logs_seen.get() != seq {
            self.logs_seen.set(seq);
            self.logs_scroll.scroll_to_bottom();
        }

        let view = cx.entity().downgrade();
        let head = header(t!("sider.logs").to_string(), None, cx)
            .child(
                h_flex()
                    .flex_none()
                    .p(px(2.))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(c.border)
                    .bg(c.bg)
                    .child(Tabs::new(
                        "logs-filter",
                        vec![
                            t!("pages.logs.all").into(),
                            "Info".into(),
                            "Warn".into(),
                            "Error".into(),
                        ],
                        FILTERS.iter().position(|f| *f == filter).unwrap_or(0),
                        move |ix, _, cx| {
                            let _ = view.update(cx, |this, cx| {
                                this.logs_filter = FILTERS[ix];
                                cx.notify();
                            });
                        },
                    )),
            )
            .child(
                Button::new("logs-clear")
                    .outline()
                    .icon(IconName::Trash)
                    .label(t!("pages.logs.clear").to_string())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.state.update(cx, |s, c| s.clear_logs(c));
                    })),
            );

        let mono = cx.theme().mono.clone();
        let rows = lines.into_iter().map(|l| {
            let (label, color) = level_style(l.level.as_ref(), cx);
            h_flex()
                .items_start()
                .gap(px(12.))
                .px(px(18.))
                .py(px(5.))
                .child(div().flex_none().text_color(c.muted).child(l.time.clone()))
                .child(div().w(px(36.)).flex_none().text_color(color).child(label))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_color(c.text2)
                        .child(crate::ui::flags::render_name(l.message.as_ref())),
                )
        });

        v_flex().size_full().child(head).child(
            div().flex_1().min_h_0().p(px(18.)).child(
                v_flex()
                    .id("logs-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.logs_scroll)
                    .py(px(8.))
                    .rounded(px(10.))
                    .border_1()
                    .border_color(c.border)
                    .bg(c.panel)
                    .font_family(mono)
                    .text_size(px(12.))
                    .children(rows),
            ),
        )
    }
}
