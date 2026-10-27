use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, Context, Hsla, IntoElement, ParentElement, SharedString, Styled, Window, div,
    px,
};
use moonglass::theme::ActiveTheme;
use moonglass::{
    Button, ButtonSize, Dropdown, IconName, MenuItem, Section, Tabs, TextField, Tone, caption,
    chip, h_flex, v_flex,
};
use rust_i18n::t;

use crate::app::state::Rule;
use crate::ui::pages::{body, header};
use crate::ui::root::{NyxApp, nyx_dialog};

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

/// Rebuilds a subscription rule string in the override `delete` format.
fn rule_to_string(r: &Rule) -> String {
    if r.kind.as_ref() == "MATCH" {
        format!("MATCH,{}", r.proxy)
    } else if r.payload.is_empty() {
        format!("{},{}", r.kind, r.proxy)
    } else {
        format!("{},{},{}", r.kind, r.payload, r.proxy)
    }
}

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

fn policy_color(p: Policy, cx: &App) -> Hsla {
    let c = cx.theme().colors;
    match p {
        Policy::Match => c.muted,
        Policy::Reject => moonglass::theme::danger(),
        Policy::Direct => c.text2,
        Policy::Proxy => c.accent,
    }
}

const COL_TYPE: f32 = 170.;
const COL_POLICY: f32 = 170.;

/// One rule as three columns: its type, what it matches, and where it sends traffic.
fn rule_cells(r: &Rule, dim: bool, cx: &App) -> Vec<AnyElement> {
    let c = cx.theme().colors;
    let mono = cx.theme().mono.clone();
    let policy = policy_of(r);
    let value = if r.payload.is_empty() {
        r.proxy.to_string()
    } else {
        r.payload.to_string()
    };
    vec![
        div()
            .w(px(COL_TYPE))
            .flex_none()
            .truncate()
            .font_family(mono.clone())
            .text_size(px(12.))
            .text_color(policy_color(policy, cx))
            .child(r.kind.to_string())
            .into_any_element(),
        div()
            .flex_1()
            .min_w_0()
            .truncate()
            .text_color(if dim { c.muted } else { c.text })
            .child(value)
            .into_any_element(),
        h_flex()
            .w(px(COL_POLICY))
            .flex_none()
            .gap(px(8.))
            .child(
                div()
                    .size(px(6.))
                    .flex_none()
                    .rounded(px(3.))
                    .bg(policy_color(policy, cx)),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_color(c.text2)
                    .child(r.proxy.to_string()),
            )
            .into_any_element(),
    ]
}

impl NyxApp {
    pub(crate) fn render_rules(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let c = cx.theme().colors;
        let st = self.state.read(cx);
        let rules = st.rules.clone();
        let has_current = st.profiles.iter().any(|p| p.is_current);
        let mode_label = match st.mode.as_ref() {
            "global" => "Global",
            "direct" => "Direct",
            _ => "Rules",
        };
        let head = header(
            t!("sider.rules").to_string(),
            Some(t!("pages.rules.summary", n => rules.len(), mode => mode_label).into()),
            cx,
        )
        .child(
            Button::new("rules-convert")
                .ghost()
                .icon(IconName::FileText)
                .tooltip(t!("pages.rules.convertMrs").to_string())
                .on_click(cx.listener(|this, _, window, cx| this.open_mrs_convert(window, cx))),
        )
        .when(has_current, |row| {
            row.child(
                Button::new("rules-edit")
                    .outline()
                    .icon(IconName::Pencil)
                    .label(t!("pages.rules.edit").to_string())
                    .on_click(cx.listener(|this, _, window, cx| this.open_rule_editor(window, cx))),
            )
        });

        let head_row = h_flex()
            .gap(px(12.))
            .px(px(18.))
            .py(px(8.))
            .font_family(cx.theme().mono.clone())
            .text_size(px(11.))
            .text_color(c.muted)
            .child(
                div()
                    .w(px(COL_TYPE))
                    .flex_none()
                    .child(t!("pages.rules.colType").to_uppercase()),
            )
            .child(
                div()
                    .flex_1()
                    .child(t!("pages.rules.colValue").to_uppercase()),
            )
            .child(
                div()
                    .w(px(COL_POLICY))
                    .flex_none()
                    .child(t!("pages.rules.colPolicy").to_uppercase()),
            );
        let table = rules
            .iter()
            .take(800)
            .fold(Section::new().row(head_row), |section, r| {
                section.row(
                    h_flex()
                        .gap(px(12.))
                        .px(px(18.))
                        .h(px(36.))
                        .children(rule_cells(r, false, cx)),
                )
            });
        v_flex()
            .size_full()
            .child(head)
            .child(body("rules-scroll").child(table))
    }

    pub(crate) fn render_mrs_dialog(
        &mut self,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let c = cx.theme().colors;
        let input_name = self
            .mrs_input
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned());
        const BEHAVIORS: [&str; 3] = ["domain", "ipcidr", "classical"];
        let selected = BEHAVIORS
            .iter()
            .position(|b| *b == self.mrs_behavior)
            .unwrap_or(0);
        let view = cx.entity().downgrade();
        nyx_dialog(
            t!("pages.rules.convertMrs").to_string(),
            v_flex()
                .gap(px(12.))
                .child(
                    h_flex()
                        .gap(px(10.))
                        .child(
                            Button::new("mrs-pick")
                                .outline()
                                .icon(IconName::Folder)
                                .label(t!("pages.rules.convertPick").to_string())
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.mrs_pick_input(window, cx)
                                })),
                        )
                        .when_some(input_name, |row, name| {
                            row.child(div().min_w_0().truncate().text_color(c.ok).child(name))
                        }),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(c.muted)
                        .child(t!("pages.rules.convertBehavior").to_string()),
                )
                .child(
                    h_flex()
                        .p(px(2.))
                        .rounded(px(8.))
                        .border_1()
                        .border_color(c.border)
                        .bg(c.bg)
                        .child(
                            Tabs::new(
                                "mrs-behavior",
                                BEHAVIORS.iter().map(|b| SharedString::from(*b)).collect(),
                                selected,
                                move |ix, _, cx| {
                                    let _ = view.update(cx, |this, cx| {
                                        this.mrs_set_behavior(BEHAVIORS[ix], cx)
                                    });
                                },
                            )
                            .fill(),
                        ),
                ),
            Some(
                h_flex()
                    .gap(px(8.))
                    .child(
                        Button::new("mrs-cancel")
                            .ghost()
                            .size(ButtonSize::Md)
                            .label(t!("common.cancel").to_string())
                            .on_click(cx.listener(|this, _, _, cx| this.close_dialog(cx))),
                    )
                    .child(
                        Button::new("mrs-go")
                            .primary()
                            .size(ButtonSize::Md)
                            .label(t!("pages.rules.convertDo").to_string())
                            .disabled(self.mrs_input.is_none())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close_dialog(cx);
                                this.submit_mrs_convert(cx)
                            })),
                    )
                    .into_any_element(),
            ),
            cx,
        )
    }

    pub(crate) fn render_rule_editor(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(re) = self.rule_editor.as_ref() else {
            return div().into_any_element();
        };
        let c = cx.theme().colors;
        let base: Vec<Rule> = self.state.read(cx).rules.clone();
        let delete: std::collections::HashSet<String> = re.delete.iter().cloned().collect();
        let prepend = re.prepend.clone();
        let append = re.append.clone();
        let custom_count = prepend.len() + append.len();

        let head = h_flex()
            .flex_none()
            .gap(px(12.))
            .px(px(18.))
            .py(px(14.))
            .border_b_1()
            .border_color(c.border)
            .child(
                Button::new("rule-ed-back")
                    .ghost()
                    .icon(IconName::ArrowLeft)
                    .tooltip(t!("common.back").to_string())
                    .on_click(cx.listener(|this, _, _, cx| this.close_rule_editor(cx))),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(6.))
                    .items_baseline()
                    .text_size(px(17.))
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::BOLD)
                            .child(t!("pages.rules.editorTitle").to_string()),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_color(c.muted)
                            .child(re.profile_name.clone()),
                    ),
            )
            .child(
                Button::new("rule-ed-save")
                    .primary()
                    .label(t!("common.save").to_string())
                    .on_click(cx.listener(|this, _, _, cx| this.save_rule_editor(cx))),
            );

        let view = cx.entity().downgrade();
        let types: Vec<MenuItem> = RULE_TYPES.iter().map(|t| MenuItem::new(*t, *t)).collect();
        let policies: Vec<MenuItem> = re
            .policies
            .iter()
            .map(|p| MenuItem::new(p.clone(), p.clone()))
            .collect();
        let type_dropdown = Dropdown::new("rule-type", types, Some(re.kind.clone()), {
            let view = view.clone();
            move |kind, window, cx| {
                let _ = view.update(cx, |this, cx| this.rule_editor_set_kind(kind, window, cx));
            }
        })
        .width(px(180.));
        let policy_dropdown = Dropdown::new("rule-policy", policies, Some(re.policy.clone()), {
            let view = view.clone();
            move |policy, _, cx| {
                let _ = view.update(cx, |this, cx| this.rule_editor_set_policy(policy, cx));
            }
        })
        .searchable(&re.policy_search)
        .width(px(220.));
        let position = h_flex()
            .flex_none()
            .p(px(2.))
            .rounded(px(8.))
            .border_1()
            .border_color(c.border)
            .bg(c.bg)
            .child(Tabs::new(
                "rule-position",
                vec![
                    t!("pages.rules.top").into(),
                    t!("pages.rules.bottom").into(),
                ],
                usize::from(re.to_append),
                move |ix, _, cx| {
                    let _ = view.update(cx, |this, cx| this.rule_editor_set_append(ix == 1, cx));
                },
            ));
        let form = Section::new().row(
            v_flex()
                .gap(px(10.))
                .px(px(14.))
                .py(px(12.))
                .child(
                    h_flex()
                        .gap(px(8.))
                        .child(type_dropdown)
                        .child(TextField::new(&re.payload).flex_1().min_w(px(120.))),
                )
                .child(
                    h_flex()
                        .gap(px(8.))
                        .child(policy_dropdown)
                        .child(position)
                        .child(div().flex_1())
                        .child(
                            Button::new("rule-ed-add")
                                .primary()
                                .icon(IconName::Plus)
                                .label(t!("pages.rules.addRule").to_string())
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.rule_editor_add(window, cx)
                                })),
                        ),
                ),
        );

        let mut list = body("rule-ed-scroll").child(form);
        if custom_count > 0 {
            let custom = prepend
                .iter()
                .enumerate()
                .map(|(i, r)| (false, i, r.clone()))
                .chain(append.iter().enumerate().map(|(i, r)| (true, i, r.clone())))
                .fold(Section::new(), |section, (is_append, i, rule)| {
                    section.row(self.rule_custom_row(rule, is_append, i, cx))
                });
            list = list
                .child(caption(
                    format!("{} ({custom_count})", t!("pages.rules.customRules")),
                    cx,
                ))
                .child(custom);
        }
        let subscription = base.iter().fold(Section::new(), |section, r| {
            let s = rule_to_string(r);
            let deleted = delete.contains(&s);
            section.row(self.rule_base_row(r, s, deleted, cx))
        });
        list = list
            .child(caption(
                format!("{} ({})", t!("pages.rules.subscriptionRules"), base.len()),
                cx,
            ))
            .child(subscription);

        v_flex()
            .size_full()
            .child(head)
            .child(list)
            .into_any_element()
    }

    fn rule_custom_row(
        &self,
        rule: String,
        append: bool,
        idx: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let badge = if append {
            t!("pages.rules.bottom")
        } else {
            t!("pages.rules.top")
        };
        h_flex()
            .gap(px(12.))
            .px(px(18.))
            .h(px(38.))
            .child(chip(badge.to_uppercase(), Tone::Accent, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .font_family(cx.theme().mono.clone())
                    .text_size(px(12.))
                    .child(rule),
            )
            .child(
                Button::new(SharedString::from(format!(
                    "rule-rm-{}-{idx}",
                    if append { "a" } else { "p" }
                )))
                .ghost()
                .size(ButtonSize::Xs)
                .icon(IconName::Trash)
                .on_click(
                    cx.listener(move |this, _, _, cx| this.rule_editor_remove(append, idx, cx)),
                ),
            )
            .into_any_element()
    }

    fn rule_base_row(
        &self,
        r: &Rule,
        rule_str: String,
        deleted: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        h_flex()
            .gap(px(12.))
            .px(px(18.))
            .h(px(38.))
            .when(deleted, |row| row.opacity(0.45))
            .children(rule_cells(r, deleted, cx))
            .child(
                Button::new(SharedString::from(format!("rule-del-{rule_str}")))
                    .ghost()
                    .size(ButtonSize::Xs)
                    .icon(if deleted {
                        IconName::Undo
                    } else {
                        IconName::Trash
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.rule_editor_toggle_delete(rule_str.clone(), cx)
                    })),
            )
            .into_any_element()
    }
}
