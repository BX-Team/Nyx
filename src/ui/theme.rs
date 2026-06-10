use gpui::{linear_color_stop, linear_gradient, rgb, rgba, App, Background};
use gpui_component::Theme;

pub const TEXT: u32 = 0xEEF3F7; // headings / primary text
pub const SUBTLE: u32 = 0xAEBCCB; // secondary text, title-bar label
pub const MUTED: u32 = 0x8DA0B0; // muted body
pub const MUTED2: u32 = 0x7D8E9D; // dimmer (mono captions)
pub const MUTED3: u32 = 0x6B7C8A; // dimmest (placeholders, icons-off)
pub const MUTED4: u32 = 0x5D6C7A; // labels / section eyebrows

pub const BG: u32 = 0x06080B; // app root
pub const WINDOW_BG: u32 = 0x0B1014; // window interior base
pub const TITLEBAR_BG: u32 = 0x0C1217;
pub const TITLEBAR_BORDER: u32 = 0x19212A;
pub const RAIL_BG: u32 = 0x080B0E; // navigation rail
pub const RAIL_BORDER: u32 = 0x19212A;
pub const CARD_BG: u32 = 0x121922; // standard card
pub const CARD_BORDER: u32 = 0x232D38;
pub const STAT_BORDER: u32 = 0x212A34; // softer card border
pub const ACTIVE_CARD_BG: u32 = 0x15202B; // selected / highlighted card
pub const ACTIVE_CARD_BORDER: u32 = 0x2C3A48;
pub const CONTROL_BG: u32 = 0x10171E; // inputs, small buttons, segmented
pub const CONTROL_BORDER: u32 = 0x222C36;
pub const PANEL_BG: u32 = 0x0A0F14; // log/console panels
pub const DIVIDER: u32 = 0x1C2630;

pub const GREEN: u32 = 0x35C97C; // primary accent
pub const GREEN_HI: u32 = 0x49DB8D; // bright text / active icons
pub const GREEN_LO: u32 = 0x26B06D; // power-button bottom / active state
pub const CYAN: u32 = 0x2AA9C9; // green→cyan gradient end
pub const ACCENT: u32 = GREEN;
pub const BLUE: u32 = 0x46B6D6; // download / info
pub const AMBER: u32 = 0xE0A030; // warning
pub const RED: u32 = 0xE0564B; // danger
pub const RED_HI: u32 = 0xEE7D72; // danger text on tint

pub const GOOD: u32 = 0x49DB8D;
pub const WARN: u32 = 0xE0A030;
pub const BAD: u32 = 0xE0564B;

// translucent green for active-nav tint / inset glow (0xRRGGBBAA)
pub const GREEN_TINT: u32 = 0x35C97C24; // ~14% fill
pub const GREEN_INSET: u32 = 0x35C97C42; // ~26% inset border
pub const GREEN_GLOW: u32 = 0x35C97C1B; // ~10% soft glow

// translucent strokes / hover overlays (white-on-dark)
pub const STROKE: u32 = 0xFFFFFF14; // ~8% white
pub const OVERLAY_SOFT: u32 = 0xFFFFFF0D; // ~5% white hover

// aliases kept for existing pages
pub const NYX_BG: u32 = CONTROL_BG;
pub const UP_COLOR: u32 = GREEN_HI;
pub const DOWN_COLOR: u32 = BLUE;

/// The atmospheric window background (faint green-tinted top fading to near-black).
pub fn content_bg() -> Background {
    linear_gradient(
        180.0,
        linear_color_stop(rgb(0x0D1613), 0.0),
        linear_color_stop(rgb(WINDOW_BG), 0.55),
    )
}

/// The bright green power-button gradient (top-light to bottom-dark).
pub fn power_on_bg() -> Background {
    linear_gradient(
        160.0,
        linear_color_stop(rgb(0x48DC8C), 0.0),
        linear_color_stop(rgb(GREEN_LO), 1.0),
    )
}

/// The green→cyan brand gradient used on the logo mark and progress fills.
pub fn brand_gradient() -> Background {
    linear_gradient(
        150.0,
        linear_color_stop(rgb(GREEN), 0.0),
        linear_color_stop(rgb(CYAN), 1.0),
    )
}

/// Overrides the gpui-component theme palette to match Nyx. Call once after
/// `gpui_component::Theme::change(Dark, …)`.
pub fn apply(cx: &mut App) {
    let t = Theme::global_mut(cx);
    let c = &mut t.colors;

    c.background = rgb(WINDOW_BG).into();
    c.foreground = rgb(TEXT).into();
    c.border = rgb(CARD_BORDER).into();
    c.muted = rgb(CARD_BG).into();
    c.muted_foreground = rgb(MUTED).into();
    c.accent = rgba(OVERLAY_SOFT).into();
    c.accent_foreground = rgb(TEXT).into();
    c.input = rgb(CONTROL_BORDER).into();
    c.ring = rgb(GREEN).into();
    c.popover = rgb(CONTROL_BG).into();
    c.popover_foreground = rgb(TEXT).into();

    c.secondary = rgb(CONTROL_BG).into();
    c.secondary_foreground = rgb(TEXT).into();
    c.secondary_hover = rgba(OVERLAY_SOFT).into();
    c.secondary_active = rgba(OVERLAY_SOFT).into();

    c.primary = rgb(GREEN).into();
    c.primary_foreground = rgb(0x0B1014).into();
    c.primary_hover = rgb(GREEN_HI).into();
    c.primary_active = rgb(GREEN_LO).into();

    c.button_primary = rgb(GREEN).into();
    c.button_primary_foreground = rgb(0x0B1014).into();
    c.button_primary_hover = rgb(GREEN_HI).into();
    c.button_primary_active = rgb(GREEN_LO).into();

    c.success = rgb(GREEN).into();
    c.success_foreground = rgb(0x0B1014).into();
    c.danger = rgb(RED).into();
    c.warning = rgb(AMBER).into();

    c.sidebar = rgb(RAIL_BG).into();
    c.sidebar_foreground = rgb(TEXT).into();
    c.sidebar_border = rgb(RAIL_BORDER).into();
    c.sidebar_accent = rgba(GREEN_TINT).into();
    c.sidebar_accent_foreground = rgb(GREEN_HI).into();
    c.sidebar_primary = rgb(GREEN).into();
    c.sidebar_primary_foreground = rgb(TEXT).into();

    c.title_bar = rgb(TITLEBAR_BG).into();
    c.title_bar_border = rgb(TITLEBAR_BORDER).into();
}
