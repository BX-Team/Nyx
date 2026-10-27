use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};
use moonglass::theme::ActiveTheme;
use moonglass::{
    Button, ButtonSize, IconName, MenuEntry, caption, h_flex, icon, motion, nav_row, v_flex,
};
use rust_i18n::t;

use crate::app::state::ProfileItem;
use crate::ui::root::{NyxApp, Route, fmt_bytes};

const SIDEBAR_WIDTH: f32 = 240.;

impl NyxApp {
    pub(crate) fn render_sidebar(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let c = cx.theme().colors;
        let profiles = self.state.read(cx).profiles.clone();
        let has_remote = profiles.iter().any(|p| p.kind.as_ref() == "remote");
        let mut pages = vec![(Route::Home, IconName::LayoutDashboard, t!("sider.home"))];
        if !profiles.is_empty() {
            pages.extend([
                (Route::Proxies, IconName::Globe, t!("sider.proxyGroup")),
                (Route::Rules, IconName::BookOpen, t!("sider.rules")),
                (
                    Route::Connections,
                    IconName::Network,
                    t!("sider.connection"),
                ),
                (Route::Logs, IconName::Terminal, t!("sider.logs")),
            ]);
        }
        let selected = pages.iter().position(|(r, ..)| *r == self.route);
        let rows: Vec<AnyElement> = pages
            .into_iter()
            .map(|(route, glyph, label)| {
                let on = self.route == route;
                nav_row(SharedString::from(format!("nav-{route:?}")), on, window, cx)
                    .child(icon(glyph, if on { c.accent } else { c.text2 }))
                    .child(div().truncate().child(label.to_string()))
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.navigate(route, window, cx)),
                    )
                    .into_any_element()
            })
            .collect();
        let nav = motion::highlighted(
            "sidebar-nav",
            v_flex().gap(px(2.)),
            rows,
            selected,
            |d| d.rounded(px(6.)).bg(c.sel),
            window,
            cx,
        );

        let mut profile_rows: Vec<AnyElement> = profiles
            .iter()
            .map(|p| self.profile_nav_row(p, window, cx))
            .collect();
        profile_rows.push(
            nav_row("new-profile", false, window, cx)
                .text_color(c.muted)
                .child(format!("+ {}", t!("pages.profiles.newProfile")))
                .on_click(cx.listener(|this, _, window, cx| this.open_profile_add(window, cx)))
                .into_any_element(),
        );

        v_flex()
            .w(px(SIDEBAR_WIDTH))
            .flex_none()
            .h_full()
            .bg(c.panel)
            .border_r_1()
            .border_color(c.border)
            .px(px(8.))
            .py(px(12.))
            .gap(px(2.))
            .child(caption(t!("sider.sections").to_string(), cx))
            .child(nav)
            .child(div().h(px(10.)))
            .child(
                h_flex()
                    .pr(px(4.))
                    .child(
                        div()
                            .flex_1()
                            .child(caption(t!("sider.profiles").to_string(), cx)),
                    )
                    .when(has_remote, |row| {
                        row.child(
                            Button::new("update-all-profiles")
                                .ghost()
                                .size(ButtonSize::Xs)
                                .icon(IconName::Refresh)
                                .tooltip(t!("pages.profiles.updateAll").to_string())
                                .on_click(
                                    cx.listener(|this, _, _, cx| this.update_all_profiles(cx)),
                                ),
                        )
                    }),
            )
            .child(
                v_flex()
                    .id("sidebar-profiles")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .gap(px(2.))
                    .children(profile_rows),
            )
            .child(
                nav_row("open-settings", false, window, cx)
                    .child(icon(IconName::Settings, c.text2))
                    .child(t!("common.settings").to_string())
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.navigate(Route::Settings, window, cx)
                    })),
            )
    }

    fn profile_nav_row(
        &self,
        p: &ProfileItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = cx.theme().colors;
        let current = p.is_current;
        let meta = if p.total > 0 {
            fmt_bytes(p.total.saturating_sub(p.used))
        } else if p.kind.as_ref() == "local" {
            t!("common.local").to_string().to_lowercase()
        } else {
            String::new()
        };
        let id = p.id.to_string();
        let entries = self.profile_menu(p, cx);
        let view = cx.entity().downgrade();
        nav_row(
            SharedString::from(format!("profile-{id}")),
            false,
            window,
            cx,
        )
        .when(current, |row| row.text_color(c.text))
        .child(
            div()
                .size(px(8.))
                .mx(px(4.))
                .flex_none()
                .rounded(px(4.))
                .map(|d| {
                    if current {
                        d.bg(c.ok)
                    } else {
                        d.border_1().border_color(c.muted)
                    }
                }),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .child(p.name.to_string()),
        )
        .when(!meta.is_empty(), |row| {
            row.child(
                div()
                    .flex_none()
                    .font_family(cx.theme().mono.clone())
                    .text_size(px(11.))
                    .text_color(c.muted)
                    .child(meta),
            )
        })
        .on_click(cx.listener(move |this, _, _, cx| {
            if !current {
                this.activate_profile(id.clone(), cx);
            }
        }))
        .on_mouse_down(MouseButton::Right, move |event, _, cx| {
            let entries = entries.clone();
            let _ = view.update(cx, |this, cx| {
                this.open_context_menu(event.position, entries, cx)
            });
        })
        .into_any_element()
    }

    /// What can be done to a profile, shared by its sidebar row and its row in Profiles.
    pub(crate) fn profile_menu(&self, p: &ProfileItem, cx: &mut Context<Self>) -> Vec<MenuEntry> {
        let view = cx.entity().downgrade();
        let id = p.id.to_string();
        let name = p.name.to_string();
        let mut entries = Vec::new();
        if !p.is_current {
            let (view, id) = (view.clone(), id.clone());
            entries.push(
                MenuEntry::action(t!("tooltips.activate").to_string(), move |_, cx| {
                    let _ = view.update(cx, |this, cx| this.activate_profile(id.clone(), cx));
                })
                .icon(IconName::Check),
            );
        }
        if p.kind.as_ref() == "remote" {
            let (view, id) = (view.clone(), id.clone());
            entries.push(
                MenuEntry::action(t!("tooltips.update").to_string(), move |_, cx| {
                    let _ = view.update(cx, |this, cx| this.update_profile(id.clone(), cx));
                })
                .icon(IconName::Refresh),
            );
        }
        {
            let (view, id) = (view.clone(), id.clone());
            entries.push(
                MenuEntry::action(
                    t!("pages.profiles.menuSubscription").to_string(),
                    move |window, cx| {
                        let _ = view.update(cx, |this, cx| {
                            this.open_profile_edit_info(id.clone(), window, cx)
                        });
                    },
                )
                .icon(IconName::Globe),
            );
        }
        {
            let (view, id) = (view.clone(), id.clone());
            entries.push(
                MenuEntry::action(
                    t!("pages.profiles.menuEditConfig").to_string(),
                    move |window, cx| {
                        let _ = view.update(cx, |this, cx| {
                            this.open_profile_editor(id.clone(), name.clone(), window, cx)
                        });
                    },
                )
                .icon(IconName::Pencil),
            );
        }
        entries.push(MenuEntry::Separator);
        entries.push(
            MenuEntry::action(t!("pages.profiles.menuDelete").to_string(), move |_, cx| {
                let _ = view.update(cx, |this, cx| this.delete_profile(id.clone(), cx));
            })
            .icon(IconName::Trash)
            .danger()
            .disabled(p.is_current),
        );
        entries
    }
}
