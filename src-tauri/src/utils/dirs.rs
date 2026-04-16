use std::path::PathBuf;

pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .map(PathBuf::from)
        .unwrap_or_default()
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Nyx")
}

pub fn app_config_path() -> PathBuf {
    data_dir().join("config.yaml")
}

pub fn controled_mihomo_config_path() -> PathBuf {
    data_dir().join("mihomo.yaml")
}

pub fn profile_config_path() -> PathBuf {
    data_dir().join("profile.yaml")
}

pub fn profiles_dir() -> PathBuf {
    data_dir().join("profiles")
}

pub fn profile_path(id: &str) -> PathBuf {
    profiles_dir().join(format!("{}.yaml", id))
}

pub fn mihomo_work_dir() -> PathBuf {
    data_dir().join("work")
}

pub fn mihomo_profile_work_dir(id: Option<&str>) -> PathBuf {
    mihomo_work_dir().join(id.unwrap_or("default"))
}

pub fn mihomo_test_dir() -> PathBuf {
    data_dir().join("test")
}

pub fn mihomo_work_config_path(id: Option<&str>) -> PathBuf {
    match id {
        Some("work") => mihomo_work_dir().join("config.yaml"),
        _ => mihomo_profile_work_dir(id).join("config.yaml"),
    }
}

pub fn log_dir() -> PathBuf {
    data_dir().join("logs")
}

pub fn log_path() -> PathBuf {
    let now = chrono::Utc::now();
    log_dir().join(format!("{}.log", now.format("%Y-%m-%d")))
}

pub fn rules_dir() -> PathBuf {
    data_dir().join("rules")
}

pub fn rule_path(id: &str) -> PathBuf {
    rules_dir().join(format!("{}.yaml", id))
}

pub fn task_dir() -> PathBuf {
    let dir = data_dir().join("tasks");
    std::fs::create_dir_all(&dir).ok();
    dir
}

pub fn mihomo_ipc_path() -> String {
    #[cfg(windows)]
    return r"\\.\pipe\Nyx\mihomo".to_string();
    #[cfg(not(windows))]
    return "/tmp/nyx-mihomo-api.sock".to_string();
}

pub fn service_ipc_path() -> String {
    #[cfg(windows)]
    return r"\\.\pipe\sparkle\service".to_string();
    #[cfg(not(windows))]
    return "/tmp/sparkle-service.sock".to_string();
}
