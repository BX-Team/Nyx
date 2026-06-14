use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

/// Nyx-owned assets embedded from `assets/` at build time (logo, extra icons, flags).
#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[include = "brand/*"]
#[include = "icons/*"]
#[include = "flags/*"]
struct NyxEmbed;

/// The asset source registered with the gpui application.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }
        if let Some(file) = NyxEmbed::get(path) {
            return Ok(Some(file.data));
        }
        gpui_component_assets::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut out: Vec<SharedString> = NyxEmbed::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect();
        out.extend(gpui_component_assets::Assets.list(path)?);
        Ok(out)
    }
}
