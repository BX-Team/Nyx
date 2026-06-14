mod connections;
mod home;
mod logs;
mod profiles;
mod proxies;
mod rules;
mod settings;

pub(crate) use rules::{rule_example, RULE_TYPES};

/// Converts a [`gpui::Keystroke`] into a `global-hotkey` accelerator string
/// (e.g. `Ctrl+Shift+KeyT`), or `None` if it can't be mapped.
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

/// Maps a gpui logical key (e.g. `t`, `1`, `f5`, `-`) to a W3C `Code` name
/// (`KeyT`, `Digit1`, `F5`, `Minus`).
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
    if let Some(n) = key.strip_prefix('f').or_else(|| key.strip_prefix('F')) {
        if let Ok(num) = n.parse::<u8>() {
            if (1..=24).contains(&num) {
                return Some(format!("F{num}"));
            }
        }
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
