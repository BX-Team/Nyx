use std::sync::Arc;

use gpui::RenderImage;

#[cfg(windows)]
mod cache {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::sync::Arc;

    use gpui::RenderImage;

    thread_local! {
        static CACHE: RefCell<HashMap<String, Option<Arc<RenderImage>>>> =
            RefCell::new(HashMap::new());
    }

    /// Returns the cached icon for `path`, extracting + caching on first use.
    pub(super) fn get(path: &str) -> Option<Arc<RenderImage>> {
        if let Some(hit) = CACHE.with(|c| c.borrow().get(path).cloned()) {
            return hit;
        }
        let icon = super::extract(path).map(Arc::new);
        CACHE.with(|c| c.borrow_mut().insert(path.to_string(), icon.clone()));
        icon
    }
}

/// The executable's icon as a gpui image, or `None` if unavailable.
#[cfg(windows)]
pub fn for_path(path: &str) -> Option<Arc<RenderImage>> {
    if path.trim().is_empty() {
        return None;
    }
    cache::get(path)
}

#[cfg(not(windows))]
pub fn for_path(_path: &str) -> Option<Arc<RenderImage>> {
    None
}

#[cfg(windows)]
fn extract(path: &str) -> Option<RenderImage> {
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    let ok = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    if ok == 0 || info.hIcon.0.is_null() {
        return None;
    }
    let hicon = info.hIcon;

    // Pull the icon's color bitmap into a top-down 32-bit BGRA buffer (the exact
    // layout gpui's `RenderImage` expects).
    let result = (|| {
        let mut icon_info: ICONINFO = unsafe { std::mem::zeroed() };
        unsafe { GetIconInfo(hicon, &mut icon_info) }.ok()?;
        let hbm_color = icon_info.hbmColor;
        let hbm_mask = icon_info.hbmMask;

        let mut bmp: BITMAP = unsafe { std::mem::zeroed() };
        let got = unsafe {
            GetObjectW(
                hbm_color.into(),
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut bmp as *mut _ as *mut _),
            )
        };
        let (w, h) = (bmp.bmWidth, bmp.bmHeight);
        let mut out = None;
        if got != 0 && w > 0 && h > 0 {
            let mut bi: BITMAPINFO = unsafe { std::mem::zeroed() };
            bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bi.bmiHeader.biWidth = w;
            bi.bmiHeader.biHeight = -h; // negative => top-down rows
            bi.bmiHeader.biPlanes = 1;
            bi.bmiHeader.biBitCount = 32;
            bi.bmiHeader.biCompression = BI_RGB.0;

            let mut buf = vec![0u8; (w * h * 4) as usize];
            let hdc = unsafe { GetDC(None) };
            let scanned = unsafe {
                GetDIBits(
                    hdc,
                    hbm_color,
                    0,
                    h as u32,
                    Some(buf.as_mut_ptr() as *mut _),
                    &mut bi,
                    DIB_RGB_COLORS,
                )
            };
            unsafe { ReleaseDC(None, hdc) };

            if scanned != 0 {
                // Some (older) icons leave the alpha channel zeroed; treat a
                // fully-transparent result as opaque so it isn't invisible.
                if buf.chunks_exact(4).all(|p| p[3] == 0) {
                    for p in buf.chunks_exact_mut(4) {
                        p[3] = 255;
                    }
                }
                if let Some(buffer) = image::RgbaImage::from_raw(w as u32, h as u32, buf) {
                    out = Some(RenderImage::new(vec![image::Frame::new(buffer)]));
                }
            }
        }
        unsafe {
            let _ = DeleteObject(hbm_color.into());
            let _ = DeleteObject(hbm_mask.into());
        }
        out
    })();

    unsafe {
        let _ = DestroyIcon(hicon);
    }
    result
}
