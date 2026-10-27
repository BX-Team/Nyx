use gpui::{App, Global, Hsla, Window};
use moonglass::theme::{self, ActiveTheme as _, Spec, ThemeMode};

/// Nyx's original green-on-ink look, kept as an extra dark theme.
pub const NYX: Spec = Spec {
    id: "nyx",
    name: "Nyx",
    colors: [
        0x0B1014, 0x080B0E, 0x1C2630, 0x121922, 0x15202B, 0xEEF3F7, 0xAEBCCB, 0x8DA0B0, 0x35C97C,
        0x0B1014, 0x49DB8D, 0xE0A030,
    ],
};

pub const DEFAULT_DARK: &str = "graphite";
pub const DEFAULT_LIGHT: &str = "day";

pub fn init(cx: &mut App) {
    moonglass::init(cx);
    theme::register(NYX, true, cx);
    cx.set_global(saved());
    apply_saved(None, cx);
}

/// The theme choice: light, dark or system, and which dark and light theme to use.
#[derive(Debug, Clone)]
pub struct Choice {
    pub mode: ThemeMode,
    pub dark: String,
    pub light: String,
}

impl Global for Choice {}

fn saved() -> Choice {
    use crate::backend::config::app_config_str;
    let mode = match app_config_str("themeMode", "system").as_str() {
        "light" => ThemeMode::Light,
        "dark" => ThemeMode::Dark,
        _ => ThemeMode::System,
    };
    Choice {
        mode,
        dark: app_config_str("darkTheme", DEFAULT_DARK),
        light: app_config_str("lightTheme", DEFAULT_LIGHT),
    }
}

pub fn choice(cx: &App) -> Choice {
    cx.global::<Choice>().clone()
}

/// Applies the current choice; also for when the system switches between light and dark.
pub fn apply_saved(window: Option<&mut Window>, cx: &mut App) {
    let c = choice(cx);
    theme::apply(c.mode, &c.dark, &c.light, window, cx);
}

/// Switches to `choice` at once and saves it.
pub fn choose(choice: Choice, cx: &mut App) {
    let patch = serde_json::json!({
        "themeMode": mode_key(choice.mode),
        "darkTheme": choice.dark,
        "lightTheme": choice.light,
    });
    crate::app::runtime::detach(async move {
        let _ = crate::backend::config::patch_app_config(patch).await;
    });
    cx.set_global(choice);
    apply_saved(None, cx);
}

pub fn mode_key(mode: ThemeMode) -> &'static str {
    match mode {
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
        ThemeMode::System => "system",
    }
}

/// Upload traffic: the theme's success color.
pub fn up_color(cx: &App) -> Hsla {
    cx.theme().colors.ok
}

/// Download traffic: the theme's accent.
pub fn down_color(cx: &App) -> Hsla {
    cx.theme().colors.accent
}

/// Latency color: fast, slow, very slow, or untested.
pub fn delay_color(delay: Option<u32>, cx: &App) -> Hsla {
    let c = cx.theme().colors;
    match delay {
        None => c.muted,
        Some(d) if d < 200 => c.ok,
        Some(d) if d < 500 => c.warn,
        Some(_) => theme::danger(),
    }
}
