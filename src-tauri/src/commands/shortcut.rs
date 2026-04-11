use tauri::AppHandle;

#[tauri::command]
pub async fn register_shortcut(
    app: AppHandle,
    old_shortcut: Option<String>,
    new_shortcut: Option<String>,
    action: String,
) -> Result<(), String> {
    crate::shortcuts::register_shortcut(
        &app,
        old_shortcut.as_deref(),
        new_shortcut.as_deref(),
        &action,
    )
    .map_err(|e| e.to_string())
}
