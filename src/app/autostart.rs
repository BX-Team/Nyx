use auto_launch::AutoLaunchBuilder;

fn builder() -> Option<auto_launch::AutoLaunch> {
    let exe = std::env::current_exe().ok()?;
    // A /nix/store path changes on every rebuild, so the autostart entry would
    // point at a garbage-collected binary; the bare name resolves via PATH.
    #[cfg(target_os = "linux")]
    let path = if exe.starts_with("/nix/store") {
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
