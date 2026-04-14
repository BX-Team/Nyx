use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn window_minimize(app: AppHandle) -> Result<(), String> {
    app.get_webview_window("main")
        .ok_or("main window not found".to_string())?
        .minimize()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn window_maximize(app: AppHandle) -> Result<(), String> {
    let win = app
        .get_webview_window("main")
        .ok_or("main window not found".to_string())?;
    if win.is_maximized().unwrap_or(false) {
        win.unmaximize()
    } else {
        win.maximize()
    }
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn window_close(app: AppHandle) -> Result<(), String> {
    app.get_webview_window("main")
        .ok_or("main window not found".to_string())?
        .close()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn window_is_maximized(app: AppHandle) -> bool {
    app.get_webview_window("main")
        .map(|w| w.is_maximized().unwrap_or(false))
        .unwrap_or(false)
}

#[tauri::command]
pub async fn show_main_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

#[tauri::command]
pub async fn close_main_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

#[tauri::command]
pub async fn trigger_main_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

#[tauri::command]
pub async fn set_always_on_top(app: AppHandle, value: bool) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_always_on_top(value);
    }
}

#[tauri::command]
pub async fn is_always_on_top(app: AppHandle) -> bool {
    app.get_webview_window("main")
        .map(|w| w.is_always_on_top().unwrap_or(false))
        .unwrap_or(false)
}

#[tauri::command]
pub async fn set_title_bar_overlay(
    app: AppHandle,
    color: Option<String>,
    _symbol_color: Option<String>,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use tauri::Manager;

        const DWMWA_CAPTION_COLOR: u32 = 35;

        if let Some(win) = app.get_webview_window("main") {
            if let Ok(hwnd) = win.hwnd() {
                if let Some(color_str) = color {
                    if let Some(hex) = color_str.strip_prefix('#') {
                        if hex.len() == 6 {
                            if let Ok(rgb) = u32::from_str_radix(hex, 16) {
                                let r = (rgb >> 16) & 0xFF;
                                let g = (rgb >> 8) & 0xFF;
                                let b = rgb & 0xFF;
                                let colorref = b << 16 | g << 8 | r;
                                unsafe {
                                    #[link(name = "dwmapi")]
                                    extern "system" {
                                        fn DwmSetWindowAttribute(
                                            hwnd: isize,
                                            dwAttribute: u32,
                                            pvAttribute: *const u32,
                                            cbAttribute: u32,
                                        ) -> i32;
                                    }
                                    let _ = DwmSetWindowAttribute(
                                        hwnd.0 as isize,
                                        DWMWA_CAPTION_COLOR,
                                        &colorref,
                                        4,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, color, _symbol_color);
    }
    Ok(())
}

#[tauri::command]
pub async fn needs_first_run_admin() -> bool {
    false
}

#[cfg(target_os = "windows")]
pub fn is_elevated_sync() -> bool {
    is_elevated()
}

#[cfg(not(target_os = "windows"))]
pub fn is_elevated_sync() -> bool {
    unsafe { libc::geteuid() == 0 }
}

#[tauri::command]
pub async fn is_admin() -> bool {
    #[cfg(target_os = "windows")]
    {
        is_elevated()
    }
    #[cfg(not(target_os = "windows"))]
    {
        unsafe { libc::geteuid() == 0 }
    }
}

#[cfg(target_os = "windows")]
fn is_elevated() -> bool {
    use std::mem;
    use std::ptr;
    unsafe {
        let mut token: *mut std::ffi::c_void = ptr::null_mut();
        #[link(name = "advapi32")]
        extern "system" {
            fn OpenProcessToken(
                process: *mut std::ffi::c_void,
                desired_access: u32,
                token_handle: *mut *mut std::ffi::c_void,
            ) -> i32;
            fn GetTokenInformation(
                token_handle: *mut std::ffi::c_void,
                token_information_class: u32,
                token_information: *mut std::ffi::c_void,
                token_information_length: u32,
                return_length: *mut u32,
            ) -> i32;
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn GetCurrentProcess() -> *mut std::ffi::c_void;
            fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        }

        const TOKEN_QUERY: u32 = 0x0008;
        const TOKEN_ELEVATION: u32 = 20; 

        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }

        #[repr(C)]
        struct TokenElevation {
            token_is_elevated: u32,
        }
        let mut elevation: TokenElevation = mem::zeroed();
        let mut size: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TOKEN_ELEVATION,
            &mut elevation as *mut _ as *mut std::ffi::c_void,
            mem::size_of::<TokenElevation>() as u32,
            &mut size,
        );
        CloseHandle(token);
        ok != 0 && elevation.token_is_elevated != 0
    }
}

#[tauri::command]
pub async fn restart_as_admin(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Start-Process -FilePath '{}' -Verb RunAs",
                    exe.display()
                ),
            ])
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;
        app.exit(0);
    }
    #[cfg(not(target_os = "windows"))]
    let _ = app;
    Ok(())
}

#[tauri::command]
pub async fn open_dev_tools(app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        win.open_devtools();
    }
}
