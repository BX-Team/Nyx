use gpui::Window;

/// Asks for client-side decorations unless the user wants the system frame.
/// gpui's X11 backend defaults to server-side, which would stack the
/// compositor's title bar on ours; it downgrades the request when no
/// compositor is present, so `NyxApp::render` follows `window_decorations()`.
pub fn request_decorations(window: &Window, system_frame: bool) {
    #[cfg(target_os = "linux")]
    window.request_decorations(if system_frame {
        gpui::WindowDecorations::Server
    } else {
        gpui::WindowDecorations::Client
    });
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (window, system_frame);
    }
}

pub fn apply_saved_decorations(window: &Window) {
    request_decorations(
        window,
        crate::backend::config::app_config_bool("useWindowFrame"),
    );
}

#[cfg(windows)]
use std::sync::atomic::{AtomicIsize, Ordering};

/// The main window's HWND, captured once at creation (gpui main thread only).
#[cfg(windows)]
static MAIN_HWND: AtomicIsize = AtomicIsize::new(0);

#[cfg(windows)]
pub fn remember(window: &Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    if let Ok(handle) = HasWindowHandle::window_handle(window)
        && let RawWindowHandle::Win32(w) = handle.as_raw()
    {
        MAIN_HWND.store(w.hwnd.get(), Ordering::SeqCst);
    }
}

#[cfg(not(windows))]
pub fn remember(_window: &Window) {}

#[cfg(windows)]
fn hwnd() -> Option<windows::Win32::Foundation::HWND> {
    let v = MAIN_HWND.load(Ordering::SeqCst);
    (v != 0).then_some(windows::Win32::Foundation::HWND(
        v as *mut core::ffi::c_void,
    ))
}

/// Hides the window to the tray. Must run outside any gpui window borrow.
#[cfg(windows)]
pub fn hide_now() {
    use windows::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};
    if let Some(h) = hwnd() {
        unsafe {
            let _ = ShowWindow(h, SW_HIDE);
        }
    }
}

/// Re-shows + foregrounds a hidden/minimized window. Borrow caveat as [`hide_now`].
#[cfg(windows)]
pub fn show_now() {
    use windows::Win32::UI::WindowsAndMessaging::{
        IsIconic, SW_RESTORE, SW_SHOW, SetForegroundWindow, ShowWindow,
    };
    if let Some(h) = hwnd() {
        unsafe {
            let cmd = if IsIconic(h).as_bool() {
                SW_RESTORE
            } else {
                SW_SHOW
            };
            let _ = ShowWindow(h, cmd);
            let _ = SetForegroundWindow(h);
        }
    }
}

/// Hides if visible and focused, else shows. Borrow caveat as [`hide_now`].
#[cfg(windows)]
pub fn toggle_now() {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, IsWindowVisible};
    if let Some(h) = hwnd() {
        unsafe {
            let visible = IsWindowVisible(h).as_bool();
            let foreground = GetForegroundWindow() == h;
            if visible && foreground {
                hide_now();
            } else {
                show_now();
            }
        }
    }
}

/// Pins/unpins the main window above all others. Borrow caveat as [`hide_now`].
#[cfg(windows)]
pub fn set_always_on_top(on: bool) {
    use windows::Win32::UI::WindowsAndMessaging::{
        HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    };
    if let Some(h) = hwnd() {
        let after = if on { HWND_TOPMOST } else { HWND_NOTOPMOST };
        unsafe {
            let _ = SetWindowPos(
                h,
                Some(after),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }
}

#[cfg(not(windows))]
pub fn set_always_on_top(_on: bool) {}

#[cfg(not(windows))]
pub fn hide(window: &Window) {
    window.minimize_window();
}
