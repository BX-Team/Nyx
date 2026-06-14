use gpui::prelude::FluentBuilder;
use gpui::{div, px, rgb, rgba, Context, InteractiveElement, IntoElement, ParentElement, Styled};
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex, v_flex, StyledExt,
};
use rust_i18n::t;
use serde_json::json;

use crate::app::runtime;
use crate::backend;
use crate::ui::root::{
    brand_gradient, NyxApp, Route, SettingsSub, BLUE, CARD_BG, CARD_BORDER, GREEN, SUBTLE, TEXT,
};

const LAST_STEP: u8 = 3;

impl NyxApp {
    pub(crate) fn onboarding_active(&self) -> bool {
        self.onboarding_step.is_some()
    }

    /// Advances the welcome flow, routing to the screen the next step is about.
    pub(crate) fn onboarding_advance(&mut self, cx: &mut Context<Self>) {
        let step = self.onboarding_step.unwrap_or(0);
        if step >= LAST_STEP {
            self.onboarding_finish(cx);
            return;
        }
        let next = step + 1;
        self.onboarding_step = Some(next);
        match next {
            1 => self.route = Route::Profiles,
            2 => {
                self.route = Route::Settings;
                self.settings_sub = Some(SettingsSub::Mihomo);
            }
            3 => self.route = Route::Home,
            _ => {}
        }
        cx.notify();
    }

    /// Dismisses the flow and records it so it never shows again.
    pub(crate) fn onboarding_finish(&mut self, cx: &mut Context<Self>) {
        self.onboarding_step = None;
        self.state.update(cx, |s, _| s.onboarding_active = false);
        runtime::detach(async {
            let _ = backend::config::patch_app_config(json!({ "onboardingDone": true })).await;
        });
        cx.notify();
    }

    pub(crate) fn render_onboarding(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
            2 if cfg!(windows) => (
                t!("onboarding.serviceTitle"),
                t!("onboarding.serviceBody"),
                Some(t!("onboarding.serviceHint")),
            ),
            2 => (
                t!("onboarding.tunTitle"),
                t!("onboarding.tunBody"),
                Some(t!("onboarding.tunHint")),
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

        let dots = h_flex().gap_1p5().children((0..=LAST_STEP).map(|i| {
            div()
                .size(px(7.))
                .rounded_full()
                .when(i == step, |d| d.bg(brand_gradient()))
                .when(i != step, |d| d.bg(rgb(CARD_BORDER)))
        }));

        let card = v_flex()
            .w(px(430.))
            .rounded_xl()
            .border_1()
            .border_color(rgb(CARD_BORDER))
            .bg(rgb(CARD_BG))
            .p_6()
            .gap_3()
            .child(
                div().text_xs().text_color(rgb(BLUE)).child(
                    t!("onboarding.step", n => step + 1, total => LAST_STEP + 1).to_string(),
                ),
            )
            .child(
                div()
                    .text_xl()
                    .font_bold()
                    .text_color(rgb(TEXT))
                    .child(title.to_string()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(SUBTLE))
                    .child(body.to_string()),
            )
            .when_some(hint, |this, h| {
                this.child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .mt_1()
                        .px(px(11.))
                        .py(px(8.))
                        .rounded(px(9.))
                        .bg(rgba(0x35C46715))
                        .border_1()
                        .border_color(rgba(0x35C46740))
                        .child(div().text_sm().text_color(rgb(GREEN)).child("→"))
                        .child(div().text_xs().text_color(rgb(GREEN)).child(h.to_string())),
                )
            })
            .child(
                h_flex()
                    .mt_2()
                    .items_center()
                    .justify_between()
                    .child(dots)
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("onb-skip")
                                    .ghost()
                                    .label(t!("onboarding.skip").to_string())
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.onboarding_finish(cx)),
                                    ),
                            )
                            .child(
                                Button::new("onb-next")
                                    .primary()
                                    .label(primary_label.to_string())
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.onboarding_advance(cx)),
                                    ),
                            ),
                    ),
            );

        // Welcome step is a centered modal; action steps float bottom-right with
        // no scrim, keeping the real UI behind clickable.
        if step == 0 {
            div()
                .id("onboarding-scrim")
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(rgba(0x000000C0))
                .child(card)
                .into_any_element()
        } else {
            div()
                .absolute()
                .bottom(px(20.))
                .right(px(20.))
                .child(card)
                .into_any_element()
        }
    }
}
