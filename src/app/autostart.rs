use auto_launch::AutoLaunchBuilder;

fn builder() -> Option<auto_launch::AutoLaunch> {
    let exe = std::env::current_exe().ok()?;
    #[cfg(target_os = "linux")]
    let path = stable_exe_path(&exe);
    #[cfg(not(target_os = "linux"))]
    let path = exe.to_string_lossy().into_owned();
    AutoLaunchBuilder::new()
        .set_app_name("Nyx")
        .set_app_path(&path)
        .build()
        .map_err(|e| log::warn!("[autostart] builder failed: {e}"))
        .ok()
}

#[cfg(target_os = "linux")]
fn stable_exe_path(exe: &std::path::Path) -> String {
    let fallback = || exe.to_string_lossy().into_owned();
    if !exe.starts_with("/nix/store") {
        return fallback();
    }
    let Some(name) = exe.file_name() else {
        return fallback();
    };
    let mut dirs_to_try = Vec::new();
    if let Ok(user) = std::env::var("USER") {
        dirs_to_try.push(std::path::PathBuf::from(format!(
            "/etc/profiles/per-user/{user}/bin"
        )));
    }
    if let Some(home) = dirs::home_dir() {
        dirs_to_try.push(home.join(".nix-profile/bin"));
    }
    dirs_to_try.push(std::path::PathBuf::from("/run/current-system/sw/bin"));

    dirs_to_try
        .into_iter()
        .map(|dir| dir.join(name))
        .find(|p| p.exists())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(fallback)
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
