use gpui::prelude::FluentBuilder;
use gpui::{
    Context, FontWeight, InteractiveElement, IntoElement, ParentElement, Styled, Window, div, hsla,
    px,
};
use moonglass::theme::ActiveTheme;
use moonglass::{Button, ButtonSize, IconName, h_flex, icon, v_flex};
use rust_i18n::t;
use serde_json::json;

use crate::app::runtime;
use crate::backend;
use crate::ui::root::{NyxApp, Route, SettingsSub};

const LAST_STEP: u8 = 3;

impl NyxApp {
    pub(crate) fn onboarding_active(&self) -> bool {
        self.onboarding_step.is_some()
    }

    pub(crate) fn onboarding_advance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let step = self.onboarding_step.unwrap_or(0);
        if step >= LAST_STEP {
            self.onboarding_finish(cx);
            return;
        }
        let next = step + 1;
        self.onboarding_step = Some(next);
        match next {
            1 => self.route = Route::Home,
            2 => {
                self.prev_route = Route::Home;
                self.route = Route::Settings;
                self.open_settings_sub(SettingsSub::Mihomo, window, cx);
            }
            3 => self.route = Route::Home,
            _ => {}
        }
        cx.notify();
    }

    pub(crate) fn onboarding_finish(&mut self, cx: &mut Context<Self>) {
        self.onboarding_step = None;
        self.state.update(cx, |s, _| s.onboarding_active = false);
        runtime::detach(async {
            let _ = backend::config::patch_app_config(json!({ "onboardingDone": true })).await;
        });
        cx.notify();
    }

    pub(crate) fn render_onboarding(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let step = self.onboarding_step.unwrap_or(0);
        let (title, body, hint) = match step {
            0 => (
                t!("onboarding.welcomeTitle"),
                t!("onboarding.welcomeBody"),
                None,
            ),
            1 => (
                t!("onboarding.profileTitle"),
                t!("onboarding.profileBody"),
                Some(t!("onboarding.profileHint")),
            ),
            2 => (
                t!("onboarding.serviceTitle"),
                t!("onboarding.serviceBody"),
                Some(t!("onboarding.serviceHint")),
            ),
            _ => (
                t!("onboarding.proxyTitle"),
                t!("onboarding.proxyBody"),
                Some(t!("onboarding.proxyHint")),
            ),
        };

        let primary_label = if step == 0 {
            t!("onboarding.start")
        } else if step >= LAST_STEP {
            t!("onboarding.finish")
        } else {
            t!("onboarding.next")
        };

        let c = cx.theme().colors;
        let dots = h_flex().gap(px(6.)).children((0..=LAST_STEP).map(|i| {
            div()
                .size(px(7.))
                .rounded(px(4.))
                .bg(if i == step { c.accent } else { c.border })
        }));

        let card = v_flex()
            .w(px(430.))
            .rounded(px(12.))
            .border_1()
            .border_color(c.border)
            .bg(c.panel)
            .shadow(vec![gpui::base::box_shadow(
                0.,
                20.,
                50.,
                0.,
                hsla(0., 0., 0., 0.5),
            )])
            .p(px(24.))
            .gap(px(12.))
            .child(
                div()
                    .font_family(cx.theme().mono.clone())
                    .text_size(px(11.))
                    .text_color(c.muted)
                    .child(
                        t!("onboarding.step", n => step + 1, total => LAST_STEP + 1).to_uppercase(),
                    ),
            )
            .child(
                div()
                    .text_size(px(20.))
                    .font_weight(FontWeight::BOLD)
                    .child(title.to_string()),
            )
            .child(
                div()
                    .text_color(c.text2)
                    .line_height(gpui::relative(1.5))
                    .child(body.to_string()),
            )
            .when_some(hint, |this, h| {
                this.child(
                    h_flex()
                        .gap(px(8.))
                        .mt(px(4.))
                        .px(px(12.))
                        .py(px(8.))
                        .rounded(px(8.))
                        .bg(c.accent.opacity(0.1))
                        .border_1()
                        .border_color(c.accent.opacity(0.3))
                        .child(icon(IconName::ChevronRight, c.accent))
                        .child(div().text_color(c.accent).child(h.to_string())),
                )
            })
            .child(
                h_flex().mt(px(8.)).justify_between().child(dots).child(
                    h_flex()
                        .gap(px(8.))
                        .child(
                            Button::new("onb-skip")
                                .ghost()
                                .size(ButtonSize::Md)
                                .label(t!("onboarding.skip").to_string())
                                .on_click(cx.listener(|this, _, _, cx| this.onboarding_finish(cx))),
                        )
                        .child(
                            Button::new("onb-next")
                                .primary()
                                .size(ButtonSize::Md)
                                .label(primary_label.to_string())
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.onboarding_advance(window, cx)
                                })),
                        ),
                ),
            );

        // Action steps float bottom-right with no scrim, keeping the UI clickable.
        if step == 0 {
            div()
                .id("onboarding-scrim")
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(hsla(0., 0., 0., 0.6))
                .child(card)
                .into_any_element()
        } else {
            div()
                .absolute()
                .bottom(px(96.))
                .right(px(20.))
                .child(card)
                .into_any_element()
        }
    }
}
