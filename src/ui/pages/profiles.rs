use gpui::prelude::FluentBuilder;
use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div, px};
use moonglass::theme::ActiveTheme;
use moonglass::{Button, ButtonSize, IconName, Tabs, TextField, h_flex, icon, v_flex};
use rust_i18n::t;

use crate::ui::root::{NyxApp, nyx_dialog};

impl NyxApp {
    pub(crate) fn render_profile_add_dialog(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = cx.theme().colors;
        let local = self.profile_add_local;
        let editing = self.profile_edit_id.is_some();
        let busy = self.profile_add_busy;
        let error = self.profile_add_error.clone();
        let file_name = self.profile_add_file.as_ref().map(|(n, _)| n.clone());
        let can_submit = if local {
            // When editing a local profile, re-picking the file is optional (bare save renames).
            editing || self.profile_add_file.is_some()
        } else {
            !self.import_url.read(cx).value().trim().is_empty()
        };
        let view = cx.entity().downgrade();
        let source_tabs = Tabs::new(
            "profadd-source",
            vec![t!("common.remote").into(), t!("common.local").into()],
            usize::from(local),
            move |ix, _, cx| {
                let _ = view.update(cx, |this, cx| this.profile_add_set_local(ix == 1, cx));
            },
        )
        .fill();

        let source = if local {
            v_flex()
                .gap(px(8.))
                .child(
                    Button::new("profadd-pick")
                        .outline()
                        .icon(IconName::Folder)
                        .label(t!("pages.profiles.selectFile").to_string())
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.profile_add_pick_file(window, cx)
                        })),
                )
                .when_some(file_name, |col, name| {
                    col.child(
                        h_flex()
                            .gap(px(6.))
                            .text_color(c.ok)
                            .child(icon(IconName::Check, c.ok).size(px(13.)))
                            .child(format!("{}: {name}", t!("pages.profiles.fileSelected"))),
                    )
                })
                .into_any_element()
        } else {
            field(
                t!("pages.profiles.urlLabel").to_string(),
                TextField::new(&self.import_url).leading(IconName::Link),
                cx,
            )
        };

        nyx_dialog(
            if editing {
                t!("pages.profiles.editTitle").to_string()
            } else {
                t!("pages.profiles.addTitle").to_string()
            },
            v_flex()
                .gap(px(14.))
                .child(
                    h_flex()
                        .p(px(2.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(c.border)
                        .bg(c.bg)
                        .child(source_tabs),
                )
                .child(source)
                .child(field(
                    t!("pages.profiles.nameLabel").to_string(),
                    TextField::new(&self.profile_add_name),
                    cx,
                ))
                .when(!local, |col| {
                    col.child(field(
                        t!("pages.profiles.intervalLabel").to_string(),
                        TextField::new(&self.profile_interval),
                        cx,
                    ))
                })
                .when_some(error, |col, e| {
                    col.child(
                        div()
                            .text_color(moonglass::theme::danger())
                            .child(format!("{}: {e}", t!("pages.profiles.importFailed"))),
                    )
                }),
            Some(
                h_flex()
                    .gap(px(8.))
                    .child(
                        Button::new("profadd-cancel")
                            .ghost()
                            .size(ButtonSize::Md)
                            .label(t!("common.cancel").to_string())
                            .disabled(busy)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.close_profile_add(window, cx)
                            })),
                    )
                    .child(
                        Button::new("profadd-submit")
                            .primary()
                            .size(ButtonSize::Md)
                            .label(if editing {
                                t!("common.save").to_string()
                            } else {
                                t!("pages.profiles.add").to_string()
                            })
                            .loading(busy)
                            .disabled(!can_submit)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.submit_profile_add(window, cx)
                            })),
                    )
                    .into_any_element(),
            ),
            cx,
        )
    }
}

/// A labelled form field in a dialog.
pub(crate) fn field(label: String, control: impl IntoElement, cx: &gpui::App) -> AnyElement {
    v_flex()
        .gap(px(6.))
        .child(
            div()
                .text_size(px(12.))
                .text_color(cx.theme().colors.muted)
                .child(label),
        )
        .child(control)
        .into_any_element()
}
