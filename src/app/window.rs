use gpui::Window;

#[cfg(windows)]
use std::sync::atomic::{AtomicIsize, Ordering};

/// The main window's HWND, captured once at creation (gpui main thread only).
#[cfg(windows)]
static MAIN_HWND: AtomicIsize = AtomicIsize::new(0);

/// Records the main window's native handle for later show/hide.
#[cfg(windows)]
pub fn remember(window: &Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    if let Ok(handle) = HasWindowHandle::window_handle(window) {
        if let RawWindowHandle::Win32(w) = handle.as_raw() {
            MAIN_HWND.store(w.hwnd.get(), Ordering::SeqCst);
        }
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

/// Hides the window from screen + taskbar (it lives on in the tray). Must run
/// outside any gpui window borrow — schedule via `App::defer`.
#[cfg(windows)]
pub fn hide_now() {
    use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
    if let Some(h) = hwnd() {
        unsafe {
            let _ = ShowWindow(h, SW_HIDE);
        }
    }
}

/// Re-shows + foregrounds a hidden/minimized window. Same borrow caveat as
/// [`hide_now`] — schedule via `App::defer`.
#[cfg(windows)]
pub fn show_now() {
    use windows::Win32::UI::WindowsAndMessaging::{
        IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
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

/// Toggles the window: if it's visible and already foreground, hide it to the
/// tray; otherwise show + foreground it. Backs the "toggle window" hotkey. Same
/// borrow caveat as [`hide_now`] / [`show_now`] — schedule via `App::spawn`.
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

/// Non-Windows fallback: minimize stands in for tray-hide.
#[cfg(not(windows))]
pub fn hide(window: &Window) {
    window.minimize_window();
}
