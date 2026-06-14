use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, rgb, rgba, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex,
    input::Input,
    select::Select,
    tooltip::Tooltip,
    v_flex, Disableable, Icon, IconName, Sizable, StyledExt,
};
use rust_i18n::t;

use crate::app::state::Rule;
use crate::ui::root::{
    NyxApp, CARD_BG, CARD_BORDER, CONTROL_BG, CONTROL_BORDER, DIVIDER, GREEN, GREEN_HI, MUTED2,
    MUTED3, MUTED4, RED, RED_HI, SUBTLE, TEXT,
};

/// Rule types offered by the smart editor's type picker (mihomo rule set).
pub(crate) const RULE_TYPES: &[&str] = &[
    "DOMAIN",
    "DOMAIN-SUFFIX",
    "DOMAIN-KEYWORD",
    "DOMAIN-REGEX",
    "GEOSITE",
    "GEOIP",
    "SRC-GEOIP",
    "IP-ASN",
    "SRC-IP-ASN",
    "IP-CIDR",
    "IP-CIDR6",
    "SRC-IP-CIDR",
    "IP-SUFFIX",
    "SRC-IP-SUFFIX",
    "SRC-PORT",
    "DST-PORT",
    "IN-PORT",
    "DSCP",
    "PROCESS-NAME",
    "PROCESS-PATH",
    "PROCESS-NAME-REGEX",
    "PROCESS-PATH-REGEX",
    "NETWORK",
    "UID",
    "IN-TYPE",
    "IN-USER",
    "IN-NAME",
    "SUB-RULE",
    "RULE-SET",
    "AND",
    "OR",
    "NOT",
    "MATCH",
];

/// Example payload placeholder for a given rule type in the "add rule" form.
pub(crate) fn rule_example(kind: &str) -> &'static str {
    match kind {
        "DOMAIN" => "example.com",
        "DOMAIN-SUFFIX" => "example.com",
        "DOMAIN-KEYWORD" => "example",
        "DOMAIN-REGEX" => "example.*",
        "GEOSITE" => "youtube",
        "GEOIP" => "CN",
        "SRC-GEOIP" => "CN",
        "IP-ASN" => "13335",
        "SRC-IP-ASN" => "9808",
        "IP-CIDR" => "127.0.0.0/8",
        "IP-CIDR6" => "2620:0:2d0:200::7/32",
        "SRC-IP-CIDR" => "192.168.1.201/32",
        "IP-SUFFIX" => "8.8.8.8/24",
        "SRC-IP-SUFFIX" => "192.168.1.201/8",
        "SRC-PORT" => "7777",
        "DST-PORT" => "80",
        "IN-PORT" => "7897",
        "DSCP" => "4",
        "PROCESS-NAME" => {
            if cfg!(windows) {
                "chrome.exe"
            } else {
                "curl"
            }
        }
        "PROCESS-PATH" => {
            if cfg!(windows) {
                "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"
            } else {
                "/usr/bin/wget"
            }
        }
        "PROCESS-NAME-REGEX" => ".*telegram.*",
        "PROCESS-PATH-REGEX" => {
            if cfg!(windows) {
                "(?i).*Application\\chrome.*"
            } else {
                ".*bin/wget"
            }
        }
        "NETWORK" => "udp",
        "UID" => "1001",
        "IN-TYPE" => "SOCKS/HTTP",
        "IN-USER" => "mihomo",
        "IN-NAME" => "ss",
        "SUB-RULE" => "(NETWORK,tcp)",
        "RULE-SET" => "providername",
        "AND" => "((DOMAIN,baidu.com),(NETWORK,UDP))",
        "OR" => "((NETWORK,UDP),(DOMAIN,baidu.com))",
        "NOT" => "((DOMAIN,baidu.com))",
        _ => "",
    }
}

/// Reconstructs the rule string for a subscription rule (matches the override
/// `delete` format), e.g. `DOMAIN-SUFFIX,example.com,DIRECT`.
fn rule_to_string(r: &Rule) -> String {
    if r.kind.as_ref() == "MATCH" {
        format!("MATCH,{}", r.proxy)
    } else if r.payload.is_empty() {
        format!("{},{}", r.kind, r.proxy)
    } else {
        format!("{},{},{}", r.kind, r.payload, r.proxy)
    }
}

/// Policy category derived from a rule's target.
#[derive(Clone, Copy, PartialEq)]
enum Policy {
    Proxy,
    Direct,
    Reject,
    Match,
}

fn policy_of(r: &Rule) -> Policy {
    if r.kind.as_ref() == "MATCH" {
        Policy::Match
    } else if r.proxy.as_ref().contains("REJECT") {
        Policy::Reject
    } else if r.proxy.as_ref() == "DIRECT" {
        Policy::Direct
    } else {
        Policy::Proxy
    }
}

/// Type-column color, keyed off the rule's policy (mirrors the mockup).
fn type_color(p: Policy) -> u32 {
    match p {
        Policy::Match => MUTED3,
        Policy::Reject => 0xD99070,
        Policy::Direct => 0x5FB6A0,
        Policy::Proxy => 0x8493DF,
    }
}

/// Policy dot + text color.
fn policy_colors(p: Policy) -> (u32, u32) {
    match p {
        Policy::Direct => (0x8493A1, SUBTLE),
        Policy::Reject => (RED, RED_HI),
        _ => (GREEN, GREEN_HI),
    }
}

const COL_TYPE: f32 = 170.;
const COL_POLICY: f32 = 150.;

impl NyxApp {
    pub(crate) fn render_rules(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let st = self.state.read(cx);
        let rules = st.rules.clone();
        let mode = st.mode.clone();
        let has_current = st.profiles.iter().any(|p| p.is_current);
        let count = rules.len();
        let mode_label = match mode.as_ref() {
            "global" => "Global",
            "direct" => "Direct",
            _ => "Rules",
        };

        let header = h_flex()
            .items_center()
            .justify_between()
            .px(px(22.))
            .pt(px(18.))
            .pb(px(14.))
            .child(
                v_flex()
                    .gap_0p5()
                    .child(
                        div()
                            .text_xl()
                            .font_bold()
                            .text_color(rgb(TEXT))
                            .child(t!("sider.rules").to_string()),
                    )
                    .child(div().text_xs().text_color(rgb(MUTED4)).child(
                        t!("pages.rules.summary", n => count, mode => mode_label).to_string(),
                    )),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .id("rules-convert")
                            .size(px(32.))
                            .rounded(px(8.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(rgb(CARD_BG))
                            .border_1()
                            .border_color(rgb(CARD_BORDER))
                            .text_color(rgb(SUBTLE))
                            .cursor_pointer()
                            .tooltip(|window, cx| {
                                Tooltip::new(t!("pages.rules.convertMrs").to_string())
                                    .build(window, cx)
                            })
                            .child(Icon::empty().path("icons/refresh.svg").size(px(15.)))
                            .on_click(cx.listener(|this, _, _, cx| this.open_mrs_convert(cx))),
                    )
                    .when(has_current, |this| {
                        this.child(
                            div()
                                .id("rules-edit")
                                .size(px(32.))
                                .rounded(px(8.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(rgb(CARD_BG))
                                .border_1()
                                .border_color(rgb(CARD_BORDER))
                                .text_color(rgb(SUBTLE))
                                .cursor_pointer()
                                .tooltip(|window, cx| {
                                    Tooltip::new(t!("pages.rules.edit").to_string())
                                        .build(window, cx)
                                })
                                .child(Icon::empty().path("icons/square-pen.svg").size(px(15.)))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_rule_editor(window, cx)
                                })),
                        )
                    }),
            );

        let table = v_flex()
            .flex_1()
            .min_h_0()
            .mx(px(22.))
            .mb(px(18.))
            .rounded(px(13.))
            .bg(rgb(CARD_BG))
            .border_1()
            .border_color(rgb(CARD_BORDER))
            .child(table_head())
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .id("rules-scroll")
                    .overflow_y_scroll()
                    .children(rules.into_iter().take(800).map(rule_row)),
            );

        v_flex().size_full().child(header).child(table)
    }

    /// The MRS converter modal: pick a `.mrs` file + behavior, write a decoded ruleset beside it.
    pub(crate) fn render_mrs_modal(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let input_name = self
            .mrs_input
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned());
        let behavior = self.mrs_behavior;

        let behavior_pill =
            |key: &'static str, label: &str, active: bool, cx: &mut Context<Self>| {
                div()
                    .id(SharedString::from(format!("mrs-bh-{key}")))
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
                    .on_click(cx.listener(move |this, _, _, cx| this.mrs_set_behavior(key, cx)))
            };

        div()
            .id("mrs-scrim")
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
                            .child(t!("pages.rules.convertMrs").to_string()),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                Button::new("mrs-pick")
                                    .small()
                                    .icon(IconName::FolderOpen)
                                    .label(t!("pages.rules.convertPick").to_string())
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.mrs_pick_input(window, cx)
                                    })),
                            )
                            .when_some(input_name, |this, name| {
                                this.child(div().text_xs().text_color(rgb(GREEN)).child(name))
                            }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(MUTED2))
                            .child(t!("pages.rules.convertBehavior").to_string()),
                    )
                    .child(
                        h_flex()
                            .p(px(3.))
                            .gap(px(2.))
                            .rounded(px(9.))
                            .bg(rgb(CONTROL_BG))
                            .border_1()
                            .border_color(rgb(CONTROL_BORDER))
                            .child(behavior_pill("domain", "domain", behavior == "domain", cx))
                            .child(behavior_pill("ipcidr", "ipcidr", behavior == "ipcidr", cx))
                            .child(behavior_pill(
                                "classical",
                                "classical",
                                behavior == "classical",
                                cx,
                            )),
                    )
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                Button::new("mrs-cancel")
                                    .ghost()
                                    .label(t!("common.cancel").to_string())
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.close_mrs_convert(cx)),
                                    ),
                            )
                            .child(
                                Button::new("mrs-go")
                                    .primary()
                                    .label(t!("pages.rules.convertDo").to_string())
                                    .disabled(self.mrs_input.is_none())
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.submit_mrs_convert(cx)),
                                    ),
                            ),
                    ),
            )
    }

    /// The smart rule-override editor (opened from the Rules page edit button).
    pub(crate) fn render_rule_editor(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(re) = self.rule_editor.as_ref() else {
            return div().into_any_element();
        };
        let base: Vec<Rule> = self.state.read(cx).rules.clone();
        let delete: std::collections::HashSet<String> = re.delete.iter().cloned().collect();
        let to_append = re.to_append;
        let prepend = re.prepend.clone();
        let append = re.append.clone();
        let custom_count = prepend.len() + append.len();

        let header = h_flex()
            .items_center()
            .justify_between()
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(
                        div()
                            .id("rule-ed-back")
                            .size(px(32.))
                            .rounded(px(8.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(rgb(CARD_BG))
                            .border_1()
                            .border_color(rgb(CARD_BORDER))
                            .text_color(rgb(SUBTLE))
                            .cursor_pointer()
                            .tooltip(|window, cx| {
                                Tooltip::new(t!("common.back").to_string()).build(window, cx)
                            })
                            .child(Icon::new(IconName::ChevronLeft).size(px(16.)))
                            .on_click(cx.listener(|this, _, _, cx| this.close_rule_editor(cx))),
                    )
                    .child(
                        v_flex()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_lg()
                                    .font_bold()
                                    .text_color(rgb(TEXT))
                                    .child(t!("pages.rules.editorTitle").to_string()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(MUTED4))
                                    .child(re.profile_name.clone()),
                            ),
                    ),
            )
            .child(
                Button::new("rule-ed-save")
                    .primary()
                    .label(t!("common.save").to_string())
                    .on_click(cx.listener(|this, _, _, cx| this.save_rule_editor(cx))),
            );

        let pos_pill = |label: String, append: bool, on: bool, cx: &mut Context<Self>| {
            div()
                .id(SharedString::from(format!("rule-pos-{label}")))
                .px(px(11.))
                .py(px(5.))
                .rounded(px(6.))
                .text_xs()
                .cursor_pointer()
                .when(on, |t| t.bg(rgb(GREEN)).text_color(rgb(0x0B1014)))
                .when(!on, |t| t.text_color(rgb(SUBTLE)))
                .child(label)
                .on_click(
                    cx.listener(move |this, _, _, cx| this.rule_editor_set_append(append, cx)),
                )
        };
        let pos_seg = h_flex()
            .p(px(3.))
            .gap(px(2.))
            .rounded(px(9.))
            .bg(rgb(CONTROL_BG))
            .border_1()
            .border_color(rgb(CONTROL_BORDER))
            .child(pos_pill(
                t!("pages.rules.top").to_string(),
                false,
                !to_append,
                cx,
            ))
            .child(pos_pill(
                t!("pages.rules.bottom").to_string(),
                true,
                to_append,
                cx,
            ));

        let form = h_flex()
            .gap_2()
            .items_center()
            .p(px(12.))
            .rounded(px(12.))
            .bg(rgb(CARD_BG))
            .border_1()
            .border_color(rgb(CARD_BORDER))
            .child(
                div()
                    .w(px(168.))
                    .child(Select::new(&re.type_select).small()),
            )
            .child(Input::new(&re.payload).flex_1())
            .child(
                div()
                    .w(px(160.))
                    .child(Select::new(&re.policy_select).small()),
            )
            .child(pos_seg)
            .child(
                Button::new("rule-ed-add")
                    .primary()
                    .icon(IconName::Plus)
                    .label(t!("pages.rules.addRule").to_string())
                    .on_click(cx.listener(|this, _, window, cx| this.rule_editor_add(window, cx))),
            );

        let mut list = v_flex()
            .flex_1()
            .min_h_0()
            .id("rule-ed-scroll")
            .overflow_y_scroll()
            .gap_2();

        if custom_count > 0 {
            list = list.child(section_label(&t!("pages.rules.customRules"), custom_count));
            for (i, r) in prepend.iter().enumerate() {
                list = list.child(self.rule_custom_row(r.clone(), false, i, cx));
            }
            for (i, r) in append.iter().enumerate() {
                list = list.child(self.rule_custom_row(r.clone(), true, i, cx));
            }
        }

        list = list.child(section_label(
            &t!("pages.rules.subscriptionRules"),
            base.len(),
        ));
        for r in base.iter() {
            let s = rule_to_string(r);
            let deleted = delete.contains(&s);
            list = list.child(self.rule_base_row(r, s, deleted, cx));
        }

        v_flex()
            .size_full()
            .p_4()
            .gap_3()
            .child(header)
            .child(form)
            .child(list)
            .into_any_element()
    }

    /// A custom (prepend/append) rule row with a remove button.
    fn rule_custom_row(
        &self,
        rule: String,
        append: bool,
        idx: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let badge = if append {
            t!("pages.rules.bottom")
        } else {
            t!("pages.rules.top")
        };
        h_flex()
            .items_center()
            .gap_3()
            .px(px(14.))
            .py(px(10.))
            .rounded(px(10.))
            .bg(rgb(CARD_BG))
            .border_1()
            .border_color(rgb(CARD_BORDER))
            .child(
                div()
                    .flex_none()
                    .px(px(6.))
                    .py(px(1.))
                    .rounded(px(5.))
                    .bg(rgb(0x163024))
                    .text_color(rgb(GREEN_HI))
                    .text_xs()
                    .child(badge.to_string().to_uppercase()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_xs()
                    .text_color(rgb(TEXT))
                    .truncate()
                    .child(rule),
            )
            .child(
                div()
                    .id(SharedString::from(format!(
                        "rule-rm-{}-{idx}",
                        if append { "a" } else { "p" }
                    )))
                    .size(px(26.))
                    .rounded(px(7.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(MUTED3))
                    .cursor_pointer()
                    .hover(|s| s.text_color(rgb(RED_HI)))
                    .child(Icon::empty().path("icons/trash-2.svg").size(px(14.)))
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.rule_editor_remove(append, idx, cx)),
                    ),
            )
            .into_any_element()
    }

    /// A read-only subscription rule row with a delete/restore toggle.
    fn rule_base_row(
        &self,
        r: &Rule,
        rule_str: String,
        deleted: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let policy = policy_of(r);
        let value = if r.payload.is_empty() {
            r.proxy.to_string()
        } else {
            r.payload.to_string()
        };
        let hover = if deleted { GREEN_HI } else { RED_HI };
        let icon = if deleted {
            Icon::new(IconName::Undo2).size(px(14.))
        } else {
            Icon::empty().path("icons/trash-2.svg").size(px(14.))
        };
        h_flex()
            .items_center()
            .gap_3()
            .px(px(14.))
            .py(px(9.))
            .rounded(px(10.))
            .bg(rgb(CARD_BG))
            .border_1()
            .border_color(rgb(CARD_BORDER))
            .when(deleted, |this| this.opacity(0.45))
            .child(
                div()
                    .w(px(150.))
                    .flex_none()
                    .text_xs()
                    .text_color(rgb(type_color(policy)))
                    .truncate()
                    .child(r.kind.to_string()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_xs()
                    .text_color(rgb(if deleted { MUTED2 } else { TEXT }))
                    .truncate()
                    .child(value),
            )
            .child(
                div()
                    .w(px(120.))
                    .flex_none()
                    .text_xs()
                    .text_color(rgb(MUTED2))
                    .truncate()
                    .child(r.proxy.to_string()),
            )
            .child(
                div()
                    .id(SharedString::from(format!("rule-del-{rule_str}")))
                    .size(px(26.))
                    .rounded(px(7.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(MUTED3))
                    .cursor_pointer()
                    .hover(move |s| s.text_color(rgb(hover)))
                    .child(icon)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.rule_editor_toggle_delete(rule_str.clone(), cx)
                    })),
            )
            .into_any_element()
    }
}

/// A small section header inside the rule editor list.
fn section_label(label: &str, count: usize) -> impl IntoElement {
    h_flex()
        .items_center()
        .gap_2()
        .pt(px(6.))
        .child(
            div()
                .text_xs()
                .font_semibold()
                .text_color(rgb(MUTED3))
                .child(label.to_string().to_uppercase()),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(MUTED4))
                .child(format!("({count})")),
        )
}

fn table_head() -> impl IntoElement {
    h_flex()
        .px(px(18.))
        .py(px(11.))
        .gap_3()
        .border_b_1()
        .border_color(rgb(DIVIDER))
        .text_color(rgb(MUTED3))
        .text_xs()
        .child(
            div()
                .w(px(COL_TYPE))
                .flex_none()
                .child(t!("pages.rules.colType").to_string().to_uppercase()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(t!("pages.rules.colValue").to_string().to_uppercase()),
        )
        .child(
            div()
                .w(px(COL_POLICY))
                .flex_none()
                .child(t!("pages.rules.colPolicy").to_string().to_uppercase()),
        )
}

fn rule_row(r: Rule) -> impl IntoElement {
    let policy = policy_of(&r);
    let (dot, text) = policy_colors(policy);
    let value = if r.payload.is_empty() {
        r.proxy.to_string()
    } else {
        r.payload.to_string()
    };

    h_flex()
        .px(px(18.))
        .py(px(11.))
        .gap_3()
        .items_center()
        .border_b_1()
        .border_color(rgb(0x161E27))
        .child(
            div()
                .w(px(COL_TYPE))
                .flex_none()
                .text_xs()
                .text_color(rgb(type_color(policy)))
                .truncate()
                .child(r.kind.to_string()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_xs()
                .text_color(rgb(TEXT))
                .truncate()
                .child(value),
        )
        .child(
            h_flex()
                .w(px(COL_POLICY))
                .flex_none()
                .gap_2()
                .items_center()
                .child(div().size(px(6.)).rounded_full().bg(rgb(dot)))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(text))
                        .truncate()
                        .child(r.proxy.to_string()),
                ),
        )
}
