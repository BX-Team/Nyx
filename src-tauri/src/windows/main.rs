use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
use std::sync::atomic::{AtomicBool, Ordering};

static SAVE_PENDING: AtomicBool = AtomicBool::new(false);

fn window_state_path() -> std::path::PathBuf {
    crate::utils::dirs::data_dir().join("window-state.json")
}

fn restore_window_state(win: &tauri::WebviewWindow) {
    let path = window_state_path();
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(state) = serde_json::from_str::<serde_json::Value>(&text) {
            if let (Some(w), Some(h)) = (state["width"].as_f64(), state["height"].as_f64()) {
                let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize::new(w, h)));
            }
            if let (Some(x), Some(y)) = (state["x"].as_f64(), state["y"].as_f64()) {
                let _ = win.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)));
            }
        }
    }
}

fn save_window_state(win: &tauri::WebviewWindow) {
    if SAVE_PENDING.swap(true, Ordering::Relaxed) {
        return; 
    }
    let win = win.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        SAVE_PENDING.store(false, Ordering::Relaxed);
        let size = win.outer_size().ok();
        let pos = win.outer_position().ok();
        let scale = win.scale_factor().unwrap_or(1.0);
        if let (Some(s), Some(p)) = (size, pos) {
            let state = serde_json::json!({
                "width": s.width as f64 / scale,
                "height": s.height as f64 / scale,
                "x": p.x as f64 / scale,
                "y": p.y as f64 / scale,
            });
            let _ = std::fs::write(window_state_path(), serde_json::to_string_pretty(&state).unwrap_or_default());
        }
    });
}

pub fn create_main_window(app: &AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    let win = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("/".into()))
        .title("Nyx")
        .inner_size(800.0, 700.0)
        .min_inner_size(600.0, 500.0)
        .decorations(false)
        .visible(false)
        .build()?;

    restore_window_state(&win);

    let app_handle = app.clone();
    let win_clone = win.clone();
    win.on_window_event(move |event| match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let _ = app_handle.emit("show-quit-confirm", ());
        }
        tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_) => {
            save_window_state(&win_clone);
        }
        _ => {}
    });

    Ok(win)
}

#[allow(dead_code)]
pub fn toggle_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}
