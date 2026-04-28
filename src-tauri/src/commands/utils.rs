use base64::Engine as _;
use tauri::AppHandle;

#[tauri::command]
pub fn get_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub fn platform() -> &'static str {
    std::env::consts::OS
}

#[tauri::command]
pub async fn get_file_path(app: AppHandle, ext: String) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;
    let path = app
        .dialog()
        .file()
        .add_filter("File", &[ext.trim_start_matches('.')])
        .blocking_pick_file();
    path.and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().to_string())
        .ok_or("No file selected".to_string())
}

#[tauri::command]
pub async fn read_text_file(file_path: String) -> Result<String, String> {
    std::fs::read_to_string(&file_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn open_file(app: AppHandle, id: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = crate::utils::dirs::profile_path(&id);
    app.opener()
        .open_path(path.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_user_agent(app: AppHandle) -> Result<String, String> {
    let version = app.package_info().version.to_string();
    Ok(format!("clash-meta/{version} mihomo/{version}"))
}

#[tauri::command]
pub async fn get_app_name(app_path: String) -> Result<String, String> {
    std::path::Path::new(&app_path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| format!("Cannot extract name from '{app_path}'"))
}

#[tauri::command]
pub async fn get_image_data_url(url: String) -> Result<String, String> {
    let resp = reqwest::get(&url).await.map_err(|e| e.to_string())?;
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/png")
        .to_string();
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{content_type};base64,{encoded}"))
}

#[cfg(windows)]
#[tauri::command]
pub async fn get_icon_data_url(app_path: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || extract_icon_png_data_url(&app_path))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(not(windows))]
#[tauri::command]
pub async fn get_icon_data_url(_app_path: String) -> Result<String, String> {
    Ok(String::new())
}

#[cfg(windows)]
fn extract_icon_png_data_url(app_path: &str) -> Result<String, String> {
    use std::path::Path;
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
        SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
    };
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
    use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, DrawIconEx, DI_NORMAL};

    if app_path.is_empty() || !Path::new(app_path).is_file() {
        return Ok(String::new());
    }

    let wide: Vec<u16> = app_path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut sfi = SHFILEINFOW::default();

    let ok = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut sfi as *mut SHFILEINFOW),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };

    if ok == 0 || sfi.hIcon.is_invalid() {
        return Ok(String::new());
    }

    let hicon = sfi.hIcon;
    let size: i32 = 32;

    let result = unsafe {
        let screen_dc = GetDC(None);
        if screen_dc.is_invalid() {
            let _ = DestroyIcon(hicon);
            return Err("GetDC failed".into());
        }
        let mem_dc = CreateCompatibleDC(Some(screen_dc));
        if mem_dc.is_invalid() {
            ReleaseDC(None, screen_dc);
            let _ = DestroyIcon(hicon);
            return Err("CreateCompatibleDC failed".into());
        }

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size,
                biHeight: -size,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bits_ptr: *mut core::ffi::c_void = std::ptr::null_mut();
        let hbitmap: HBITMAP =
            match CreateDIBSection(Some(mem_dc), &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0) {
                Ok(b) => b,
                Err(e) => {
                    let _ = DeleteDC(mem_dc);
                    ReleaseDC(None, screen_dc);
                    let _ = DestroyIcon(hicon);
                    return Err(e.to_string());
                }
            };

        let old = SelectObject(mem_dc, HGDIOBJ(hbitmap.0));
        let draw_ok = DrawIconEx(mem_dc, 0, 0, hicon, size, size, 0, None, DI_NORMAL).is_ok();

        let byte_count = (size * size * 4) as usize;
        let mut pixels = vec![0u8; byte_count];
        if draw_ok && !bits_ptr.is_null() {
            std::ptr::copy_nonoverlapping(bits_ptr as *const u8, pixels.as_mut_ptr(), byte_count);
        }

        SelectObject(mem_dc, old);
        let _ = DeleteObject(HGDIOBJ(hbitmap.0));
        let _ = DeleteDC(mem_dc);
        ReleaseDC(None, screen_dc);
        let _ = DestroyIcon(hicon);

        if !draw_ok {
            return Err("DrawIconEx failed".into());
        }
        pixels
    };

    let mut pixels = result;
    for px in pixels.chunks_exact_mut(4) {
        px.swap(0, 2);
    }

    let mut png_bytes: Vec<u8> = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, size as u32, size as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer
            .write_image_data(&pixels)
            .map_err(|e| e.to_string())?;
    }

    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
    Ok(format!("data:image/png;base64,{b64}"))
}

#[tauri::command]
pub async fn alert(_app: AppHandle, msg: String) {
    log::warn!("alert: {msg}");
}

#[tauri::command]
pub async fn reset_app_config(app: AppHandle) -> Result<(), String> {
    let data_dir = crate::utils::dirs::data_dir();
    if data_dir.exists() {
        std::fs::remove_dir_all(&data_dir).map_err(|e| e.to_string())?;
    }
    app.restart();
}

#[tauri::command]
pub async fn relaunch_app(app: AppHandle) {
    app.restart();
}

#[tauri::command]
pub async fn quit_without_core(app: AppHandle) -> Result<(), String> {
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn quit_app(app: AppHandle) {
    #[cfg(windows)]
    let _ = crate::commands::service::stop_service().await;

    let _ = crate::core::manager::stop_core().await;
    app.exit(0);
}

#[tauri::command]
pub async fn not_dialog_quit(app: AppHandle) {
    #[cfg(windows)]
    let _ = crate::commands::service::stop_service().await;

    let _ = crate::core::manager::stop_core().await;
    app.exit(0);
}

#[tauri::command]
pub async fn debug_info() -> Result<serde_json::Value, String> {
    let data_dir = crate::utils::dirs::data_dir();
    let controller_url = crate::core::manager::controller_url();
    let core_installed = crate::core::manager::core_installed().await;

    let core_reachable = if !controller_url.is_empty() {
        let version_url = format!("{}/version", controller_url);
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .unwrap_or_default()
            .get(&version_url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    } else {
        false
    };

    let overrides_path = crate::utils::dirs::controled_mihomo_config_path();
    let overrides_content = tokio::fs::read_to_string(&overrides_path)
        .await
        .unwrap_or_else(|_| "<not found>".to_string());

    let app_config_path = crate::utils::dirs::app_config_path();
    let app_config_content = tokio::fs::read_to_string(&app_config_path)
        .await
        .unwrap_or_else(|_| "<not found>".to_string());

    let cm = mihomo_rs::ConfigManager::with_home(data_dir.clone()).ok();
    let running_config_content = if let Some(ref cm) = cm {
        if let Ok(path) = cm.get_current_path().await {
            tokio::fs::read_to_string(&path)
                .await
                .unwrap_or_else(|_| "<not found>".to_string())
        } else {
            "<cm.get_current_path failed>".to_string()
        }
    } else {
        "<ConfigManager init failed>".to_string()
    };

    let api_version = if core_reachable {
        let version_url = format!("{}/version", controller_url);
        match reqwest::get(&version_url).await {
            Ok(r) => r.text().await.unwrap_or_else(|_| "<error>".to_string()),
            Err(e) => format!("<error: {e}>"),
        }
    } else {
        "<not reachable>".to_string()
    };

    Ok(serde_json::json!({
        "data_dir": data_dir.to_string_lossy(),
        "controller_url": controller_url,
        "core_installed": core_installed,
        "core_reachable": core_reachable,
        "api_version": api_version,
        "overrides_yaml": overrides_content,
        "app_config_yaml": app_config_content,
        "running_config_yaml": running_config_content,
    }))
}
