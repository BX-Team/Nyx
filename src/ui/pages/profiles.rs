use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, rgb, rgba, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex,
    input::Input,
    tooltip::Tooltip,
    v_flex, Disableable, Icon, IconName, Sizable, StyledExt,
};
use rust_i18n::t;

use crate::app::state::ProfileItem;
use crate::ui::root::{
    brand_gradient, fmt_bytes, NyxApp, ACTIVE_CARD_BG, ACTIVE_CARD_BORDER, BLUE, CARD_BG,
    CARD_BORDER, CONTROL_BG, CONTROL_BORDER, DIVIDER, GREEN, MUTED, MUTED2, SUBTLE, TEXT,
};

/// Days until `expire` (unix ts), or a localized "never" when unset.
fn expiry_label(expire: i64) -> String {
    if expire <= 0 {
        return t!("pages.home.never").to_string();
    }
    let now = chrono::Utc::now().timestamp();
    let days = ((expire - now) / 86_400).max(0);
    t!("pages.profiles.days", n => days).to_string()
}

impl NyxApp {
    pub(crate) fn render_profiles(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let profiles = self.state.read(cx).profiles.clone();
        let count = profiles.len();

        let header = h_flex()
            .items_center()
            .justify_between()
            .px(px(22.))
            .pt(px(18.))
            .pb(px(16.))
            .child(
                v_flex()
                    .gap_0p5()
                    .child(
                        div()
                            .text_xl()
                            .font_bold()
                            .text_color(rgb(TEXT))
                            .child(t!("sider.profileManagement").to_string()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED2))
                            .child(t!("pages.profiles.count", n => count).to_string()),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("update-all")
                            .icon(Icon::empty().path("icons/refresh.svg"))
                            .label(t!("pages.profiles.updateAll").to_string())
                            .on_click(cx.listener(|this, _, _, cx| this.update_all_profiles(cx))),
                    )
                    .child(
                        Button::new("add-file")
                            .primary()
                            .icon(IconName::Plus)
                            .label(t!("pages.profiles.add").to_string())
                            .on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.open_profile_add(window, cx)
                                }),
                            ),
                    ),
            );

        let list = if profiles.is_empty() {
            v_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_color(rgb(MUTED))
                        .child(t!("pages.profiles.empty").to_string()),
                )
                .into_any_element()
        } else {
            v_flex()
                .flex_1()
                .min_h_0()
                .px(px(22.))
                .pb(px(18.))
                .id("profiles-scroll")
                .overflow_y_scroll()
                .gap_3()
                .children(
                    profiles
                        .into_iter()
                        .map(|p| self.render_profile_card(p, cx)),
                )
                .into_any_element()
        };

        v_flex().size_full().child(header).child(list)
    }

    fn render_profile_card(&self, p: ProfileItem, cx: &mut Context<Self>) -> impl IntoElement {
        let is_remote = p.kind.as_ref() == "remote";
        let current = p.is_current;
        let id = p.id.to_string();
        let name = p.name.to_string();

        let mut chips = h_flex().gap_2().items_center().child(
            div()
                .text_base()
                .font_semibold()
                .text_color(rgb(TEXT))
                .child(name.clone()),
        );
        if current {
            chips = chips.child(chip(&t!("pages.profiles.active"), true));
        }
        chips = chips.child(chip(&p.kind.to_uppercase(), false));
        if p.interval > 0 {
            chips = chips.child(chip(
                &t!("pages.profiles.autoEvery", n => p.interval / 60),
                false,
            ));
        }

        let middle = v_flex()
            .flex_1()
            .min_w_0()
            .gap_2()
            .child(chips)
            .when(p.total > 0, |this| {
                let pct = ((p.used as f64 / p.total as f64) * 100.0).clamp(0.0, 100.0);
                this.child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        .child(
                            div().flex_1().max_w(px(320.)).child(
                                div()
                                    .h(px(6.))
                                    .w_full()
                                    .rounded(px(4.))
                                    .bg(rgb(DIVIDER))
                                    .child(
                                        div()
                                            .h_full()
                                            .w(gpui::relative((pct / 100.0) as f32))
                                            .rounded(px(4.))
                                            .when(current, |b| b.bg(brand_gradient()))
                                            .when(!current, |b| b.bg(rgb(BLUE))),
                                    ),
                            ),
                        )
                        .child(div().text_xs().text_color(rgb(MUTED2)).child(format!(
                            "{} / {}",
                            fmt_bytes(p.used),
                            fmt_bytes(p.total)
                        ))),
                )
            });

        let expiry = v_flex()
            .items_end()
            .gap_0p5()
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(MUTED2))
                    .child(t!("pages.profiles.expires").to_string()),
            )
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .text_color(rgb(SUBTLE))
                    .child(expiry_label(p.expire)),
            );

        h_flex()
            .items_center()
            .gap_4()
            .p(px(17.))
            .rounded(px(14.))
            .bg(rgb(if current { ACTIVE_CARD_BG } else { CARD_BG }))
            .border_1()
            .border_color(rgb(if current {
                ACTIVE_CARD_BORDER
            } else {
                CARD_BORDER
            }))
            .child(middle)
            .child(expiry)
            .child(self.profile_actions(&id, &name, current, is_remote, cx))
    }

    /// The per-card icon action cluster (activate / refresh / edit / delete).
    fn profile_actions(
        &self,
        id: &str,
        name: &str,
        current: bool,
        is_remote: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut row = h_flex().gap_1();
        if !current {
            let aid = id.to_string();
            row = row.child(
                icon_btn(&format!("act-{id}"), Icon::new(IconName::Check), false)
                    .tooltip(|window, cx| {
                        Tooltip::new(t!("tooltips.activate").to_string()).build(window, cx)
                    })
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.activate_profile(aid.clone(), cx)),
                    ),
            );
        }
        if is_remote {
            let uid = id.to_string();
            row = row.child(
                icon_btn(
                    &format!("upd-{id}"),
                    Icon::empty().path("icons/refresh.svg"),
                    false,
                )
                .tooltip(|window, cx| {
                    Tooltip::new(t!("tooltips.update").to_string()).build(window, cx)
                })
                .on_click(cx.listener(move |this, _, _, cx| this.update_profile(uid.clone(), cx))),
            );
        }
        let iid = id.to_string();
        row = row.child(
            icon_btn(
                &format!("info-{id}"),
                Icon::empty().path("icons/link.svg"),
                false,
            )
            .tooltip(|window, cx| {
                Tooltip::new(t!("tooltips.editInfo").to_string()).build(window, cx)
            })
            .on_click(cx.listener(move |this, _, window, cx| {
                this.open_profile_edit_info(iid.clone(), window, cx)
            })),
        );
        let eid = id.to_string();
        let ename = name.to_string();
        row = row.child(
            icon_btn(
                &format!("edit-{id}"),
                Icon::empty().path("icons/square-pen.svg"),
                false,
            )
            .tooltip(|window, cx| Tooltip::new(t!("tooltips.edit").to_string()).build(window, cx))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.open_profile_editor(eid.clone(), ename.clone(), window, cx)
            })),
        );
        let del = icon_btn(
            &format!("del-{id}"),
            Icon::empty().path("icons/trash-2.svg"),
            current,
        );
        let del = if current {
            del
        } else {
            let did = id.to_string();
            del.tooltip(|window, cx| {
                Tooltip::new(t!("tooltips.delete").to_string()).build(window, cx)
            })
            .on_click(cx.listener(move |this, _, _, cx| this.delete_profile(did.clone(), cx)))
        };
        row.child(del)
    }

    /// The "Add subscription" modal: Remote/Local toggle, URL or file picker, name, Import/Cancel.
    pub(crate) fn render_profile_add_modal(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let local = self.profile_add_local;
        let editing = self.profile_edit_id.is_some();
        let file_name = self.profile_add_file.as_ref().map(|(n, _)| n.clone());
        let can_submit = if local {
            // When editing a local profile, re-picking the file is optional (bare save renames).
            editing || self.profile_add_file.is_some()
        } else {
            !self.import_url.read(cx).value().trim().is_empty()
        };

        let toggle = h_flex()
            .p(px(3.))
            .gap(px(2.))
            .rounded(px(9.))
            .bg(rgb(CONTROL_BG))
            .border_1()
            .border_color(rgb(CONTROL_BORDER))
            .child(source_pill(
                &t!("common.remote"),
                !local,
                cx.listener(|this, _, _, cx| this.profile_add_set_local(false, cx)),
            ))
            .child(source_pill(
                &t!("common.local"),
                local,
                cx.listener(|this, _, _, cx| this.profile_add_set_local(true, cx)),
            ));

        let source = if local {
            let picked = file_name.is_some();
            v_flex()
                .gap_2()
                .child(
                    Button::new("profadd-pick")
                        .small()
                        .icon(IconName::FolderOpen)
                        .label(t!("pages.profiles.selectFile").to_string())
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.profile_add_pick_file(window, cx)
                        })),
                )
                .when(picked, |this| {
                    this.child(
                        h_flex()
                            .gap_1p5()
                            .items_center()
                            .text_xs()
                            .text_color(rgb(GREEN))
                            .child(Icon::new(IconName::Check).size(px(13.)))
                            .child(format!(
                                "{}: {}",
                                t!("pages.profiles.fileSelected"),
                                file_name.unwrap_or_default()
                            )),
                    )
                })
                .into_any_element()
        } else {
            field_label(&t!("pages.profiles.urlLabel"))
                .child(Input::new(&self.import_url))
                .into_any_element()
        };

        div()
            .id("profadd-scrim")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x000000B0))
            .child(
                v_flex()
                    .w(px(440.))
                    .rounded_xl()
                    .border_1()
                    .border_color(rgb(CARD_BORDER))
                    .bg(rgb(CARD_BG))
                    .p_5()
                    .gap_3()
                    .child(
                        div()
                            .text_lg()
                            .font_bold()
                            .text_color(rgb(TEXT))
                            .child(if editing {
                                t!("pages.profiles.editTitle").to_string()
                            } else {
                                t!("pages.profiles.addTitle").to_string()
                            }),
                    )
                    .child(toggle)
                    .child(source)
                    .child(
                        field_label(&t!("pages.profiles.nameLabel"))
                            .child(Input::new(&self.profile_add_name)),
                    )
                    .when(!local, |this| {
                        this.child(
                            field_label(&t!("pages.profiles.intervalLabel"))
                                .child(Input::new(&self.profile_interval)),
                        )
                    })
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("profadd-cancel")
                                    .ghost()
                                    .label(t!("common.cancel").to_string())
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.close_profile_add(cx)),
                                    ),
                            )
                            .child(
                                Button::new("profadd-import")
                                    .primary()
                                    .label(if editing {
                                        t!("common.save").to_string()
                                    } else {
                                        t!("pages.profiles.add").to_string()
                                    })
                                    .disabled(!can_submit)
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.submit_profile_add(cx)),
                                    ),
                            ),
                    ),
            )
    }
}

/// A small labeled column wrapper for a modal form field.
fn field_label(label: &str) -> gpui::Div {
    v_flex().gap_1p5().child(
        div()
            .text_xs()
            .text_color(rgb(MUTED2))
            .child(label.to_string()),
    )
}

/// A Remote/Local source toggle pill.
fn source_pill(
    label: &str,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("src-{label}")))
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .py(px(5.))
        .rounded(px(6.))
        .text_xs()
        .cursor_pointer()
        .when(active, |this| this.bg(rgb(GREEN)).text_color(rgb(0x0B1014)))
        .when(!active, |this| this.text_color(rgb(SUBTLE)))
        .child(label.to_string())
        .on_click(on_click)
}

/// A small uppercase chip (`ACTIVE`, `REMOTE`, …).
fn chip(label: &str, accent: bool) -> impl IntoElement {
    div()
        .px(px(7.))
        .py(px(2.))
        .rounded(px(5.))
        .text_xs()
        .when(accent, |this| this.bg(rgb(GREEN)).text_color(rgb(0x0B1014)))
        .when(!accent, |this| {
            this.bg(rgb(0x1A2530)).text_color(rgb(SUBTLE))
        })
        .child(label.to_string())
}

/// A 32px bordered icon button used in the card action cluster.
fn icon_btn(key: &str, icon: Icon, disabled: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(SharedString::from(format!("prof-{key}")))
        .size(px(32.))
        .rounded(px(8.))
        .flex()
        .items_center()
        .justify_center()
        .bg(rgb(CONTROL_BG))
        .border_1()
        .border_color(rgb(CONTROL_BORDER))
        .text_color(rgb(if disabled { DIVIDER } else { SUBTLE }))
        .when(!disabled, |this| this.cursor_pointer())
        .child(icon.size(px(15.)))
}
