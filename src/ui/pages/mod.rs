mod connections;
mod home;
mod logs;
mod profiles;
mod proxies;
mod rules;
mod settings;

pub(crate) use rules::{RULE_TYPES, rule_example};

use gpui::{App, Div, FontWeight, ParentElement, SharedString, Stateful, Styled, div, px};
use moonglass::theme::ActiveTheme;
use moonglass::{h_flex, v_flex};

/// A screen's top row: its title and a muted note; callers append tabs and buttons.
pub(crate) fn header(title: impl Into<SharedString>, meta: Option<SharedString>, cx: &App) -> Div {
    let c = cx.theme().colors;
    h_flex()
        .flex_none()
        .gap(px(12.))
        .px(px(18.))
        .py(px(14.))
        .border_b_1()
        .border_color(c.border)
        .child(
            h_flex()
                .flex_1()
                .min_w_0()
                .gap(px(6.))
                .items_baseline()
                .text_size(px(17.))
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .truncate()
                        .child(title.into()),
                )
                .children(meta.map(|m| div().flex_none().text_color(c.muted).child(m))),
        )
}

/// The scrolling body under a [`header`].
pub(crate) fn body(id: &'static str) -> Stateful<Div> {
    use gpui::{InteractiveElement, StatefulInteractiveElement};
    v_flex()
        .id(id)
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .p(px(18.))
        .gap(px(14.))
}

/// Up/down traffic in the bar's monospace style.
pub(crate) fn updown(up: u64, down: u64, cx: &App) -> Div {
    h_flex()
        .flex_none()
        .gap(px(10.))
        .font_family(cx.theme().mono.clone())
        .text_size(px(12.))
        .child(
            div()
                .text_color(crate::ui::theme::down_color(cx))
                .child(format!("↓ {}", crate::ui::root::fmt_bytes(down))),
        )
        .child(
            div()
                .text_color(crate::ui::theme::up_color(cx))
                .child(format!("↑ {}", crate::ui::root::fmt_bytes(up))),
        )
}

/// A [`gpui::Keystroke`] as a `global-hotkey` accelerator (`Ctrl+Shift+KeyT`).
pub(crate) fn keystroke_to_accel(ks: &gpui::Keystroke) -> Option<String> {
    let code = key_to_code(&ks.key)?;
    let m = &ks.modifiers;
    let mut parts: Vec<&str> = Vec::new();
    if m.control {
        parts.push("Ctrl");
    }
    if m.alt {
        parts.push("Alt");
    }
    if m.shift {
        parts.push("Shift");
    }
    if m.platform {
        parts.push("Super");
    }
    let mut accel = parts.join("+");
    if !accel.is_empty() {
        accel.push('+');
    }
    accel.push_str(&code);
    Some(accel)
}

/// A gpui logical key to its W3C `Code` name (`t` → `KeyT`, `1` → `Digit1`).
fn key_to_code(key: &str) -> Option<String> {
    if key.len() == 1 {
        let c = key.chars().next().unwrap();
        if c.is_ascii_alphabetic() {
            return Some(format!("Key{}", c.to_ascii_uppercase()));
        }
        if c.is_ascii_digit() {
            return Some(format!("Digit{c}"));
        }
        return match c {
            '-' => Some("Minus".into()),
            '=' => Some("Equal".into()),
            '[' => Some("BracketLeft".into()),
            ']' => Some("BracketRight".into()),
            ';' => Some("Semicolon".into()),
            '\'' => Some("Quote".into()),
            ',' => Some("Comma".into()),
            '.' => Some("Period".into()),
            '/' => Some("Slash".into()),
            '`' => Some("Backquote".into()),
            '\\' => Some("Backslash".into()),
            _ => None,
        };
    }
    if let Some(n) = key.strip_prefix('f').or_else(|| key.strip_prefix('F'))
        && let Ok(num) = n.parse::<u8>()
        && (1..=24).contains(&num)
    {
        return Some(format!("F{num}"));
    }
    match key {
        "space" => Some("Space".into()),
        "enter" => Some("Enter".into()),
        "tab" => Some("Tab".into()),
        "up" => Some("ArrowUp".into()),
        "down" => Some("ArrowDown".into()),
        "left" => Some("ArrowLeft".into()),
        "right" => Some("ArrowRight".into()),
        "home" => Some("Home".into()),
        "end" => Some("End".into()),
        "pageup" => Some("PageUp".into()),
        "pagedown" => Some("PageDown".into()),
        "insert" => Some("Insert".into()),
        _ => None,
    }
}

/// Prettifies an accelerator for display (`Ctrl+Shift+KeyT` → `Ctrl+Shift+T`).
pub(crate) fn pretty_accel(accel: &str) -> String {
    accel
        .split('+')
        .map(|part| {
            if let Some(rest) = part.strip_prefix("Key") {
                rest.to_string()
            } else if let Some(rest) = part.strip_prefix("Digit") {
                rest.to_string()
            } else if part == "Super" {
                if cfg!(windows) {
                    "Win".into()
                } else {
                    "Super".into()
                }
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}
