mod bar;
pub(crate) mod flags;
mod onboarding;
mod pages;
mod root;
mod sidebar;
pub(crate) mod theme;

#[cfg(not(windows))]
pub(crate) use root::save_main_window_bounds;
pub use root::{init_keys, open_main_window};

/// Gives the moonglass components their words in the current language.
pub fn set_labels(cx: &mut gpui::App) {
    moonglass::set_labels(
        moonglass::Labels {
            clear: rust_i18n::t!("common.clear").to_string().into(),
        },
        cx,
    );
}
