use auto_launch::AutoLaunchBuilder;

fn builder() -> Option<auto_launch::AutoLaunch> {
    let exe = std::env::current_exe().ok()?;
    #[cfg(target_os = "linux")]
    let path = if crate::backend::elevation::is_nixos() && exe.starts_with("/nix/store") {
        "nyx".to_string()
    } else {
        exe.to_string_lossy().into_owned()
    };
    #[cfg(not(target_os = "linux"))]
    let path = exe.to_string_lossy().into_owned();
    AutoLaunchBuilder::new()
        .set_app_name("Nyx")
        .set_app_path(&path)
        .build()
        .map_err(|e| log::warn!("[autostart] builder failed: {e}"))
        .ok()
}

/// Enables or disables launch-on-login to match `enabled`.
pub fn set(enabled: bool) {
    let Some(auto) = builder() else {
        return;
    };
    let res = if enabled {
        auto.enable()
    } else {
        auto.disable()
    };
    if let Err(e) = res {
        log::warn!("[autostart] set({enabled}) failed: {e}");
    }
}

/// Reconciles the OS autostart entry with the desired flag (called on startup).
pub fn sync(enabled: bool) {
    let Some(auto) = builder() else {
        return;
    };
    match auto.is_enabled() {
        Ok(cur) if cur != enabled => set(enabled),
        Ok(_) => {}
        Err(e) => log::warn!("[autostart] is_enabled failed: {e}"),
    }
}
