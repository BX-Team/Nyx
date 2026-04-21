use anyhow::Result;
use mihomo_rs::{Channel, ConfigManager, VersionManager};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::path::PathBuf;
use tokio::sync::Mutex as AsyncMutex;

static CONTROLLER_URL: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));

static REBUILD_LOCK: Lazy<AsyncMutex<()>> = Lazy::new(|| AsyncMutex::new(()));

fn vm() -> Result<VersionManager> {
    VersionManager::with_home(crate::utils::dirs::data_dir()).map_err(|e| anyhow::anyhow!("{e}"))
}

fn cm() -> Result<ConfigManager> {
    ConfigManager::with_home(crate::utils::dirs::data_dir()).map_err(|e| anyhow::anyhow!("{e}"))
}

fn version_matches_channel(version: &str, want_alpha: bool) -> bool {
    let lower = version.to_ascii_lowercase();
    let is_alpha = lower.contains("alpha")
        || lower.contains("preview")
        || lower.contains("pre")
        || lower.contains("nightly");
    if want_alpha {
        is_alpha
    } else {
        !is_alpha
    }
}

fn pid_file() -> PathBuf {
    crate::utils::dirs::data_dir().join("mihomo.pid")
}

async fn spawn_mihomo(binary: &std::path::Path, config: &std::path::Path) -> Result<u32> {
    let mut cmd = std::process::Command::new(binary);
    cmd.arg("-d")
        .arg(
            config
                .parent()
                .ok_or_else(|| anyhow::anyhow!("config has no parent dir"))?,
        )
        .arg("-f")
        .arg(config)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("failed to spawn mihomo: {e}"))?;
    let pid = child.id();
    std::mem::forget(child);

    let pf = pid_file();
    if let Some(p) = pf.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    tokio::fs::write(&pf, pid.to_string())
        .await
        .map_err(|e| anyhow::anyhow!("failed to write PID file: {e}"))?;

    Ok(pid)
}

async fn stop_mihomo() -> Result<()> {
    let pf = pid_file();
    if !pf.exists() {
        return Ok(());
    }
    let content = match tokio::fs::read_to_string(&pf).await {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };
    let pid: u32 = match content.trim().parse() {
        Ok(p) => p,
        Err(_) => {
            let _ = tokio::fs::remove_file(&pf).await;
            return Ok(());
        }
    };

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(0x08000000)
            .output();
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output();
    }

    let _ = tokio::fs::remove_file(&pf).await;
    Ok(())
}

async fn read_current_profile_id() -> Option<String> {
    let meta_path = crate::utils::dirs::profile_config_path();
    if !meta_path.exists() {
        return None;
    }
    let meta_str = tokio::fs::read_to_string(&meta_path).await.ok()?;
    let meta: serde_yaml::Value = serde_yaml::from_str(&meta_str).ok()?;
    meta.get("current")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

async fn read_current_profile_yaml() -> String {
    use crate::utils::dirs;

    let current_id = match read_current_profile_id().await {
        Some(id) => id,
        None => return String::new(),
    };
    let profile_path = dirs::profile_path(&current_id);
    tokio::fs::read_to_string(&profile_path)
        .await
        .unwrap_or_default()
}

async fn apply_rule_overrides(yaml: &str, profile_id: &str) -> String {
    let rule_path = crate::utils::dirs::rule_path(profile_id);
    if !rule_path.exists() {
        return yaml.to_string();
    }

    let rule_content = match tokio::fs::read_to_string(&rule_path).await {
        Ok(s) => s,
        Err(_) => return yaml.to_string(),
    };

    let rule_val: serde_yaml::Value = match serde_yaml::from_str(&rule_content) {
        Ok(v) => v,
        Err(_) => return yaml.to_string(),
    };

    let prepend: Vec<serde_yaml::Value> = rule_val
        .get("prepend")
        .and_then(|v| v.as_sequence())
        .cloned()
        .unwrap_or_default();
    let append: Vec<serde_yaml::Value> = rule_val
        .get("append")
        .and_then(|v| v.as_sequence())
        .cloned()
        .unwrap_or_default();
    let delete: Vec<serde_yaml::Value> = rule_val
        .get("delete")
        .and_then(|v| v.as_sequence())
        .cloned()
        .unwrap_or_default();

    if prepend.is_empty() && append.is_empty() && delete.is_empty() {
        return yaml.to_string();
    }

    let mut config_val: serde_yaml::Value = match serde_yaml::from_str(yaml) {
        Ok(v) => v,
        Err(_) => return yaml.to_string(),
    };

    if let serde_yaml::Value::Mapping(ref mut map) = config_val {
        let rules_key = serde_yaml::Value::String("rules".to_string());
        let existing_rules: Vec<serde_yaml::Value> = map
            .get(&rules_key)
            .and_then(|v| v.as_sequence())
            .cloned()
            .unwrap_or_default();

        let delete_set: std::collections::HashSet<String> = delete
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect();

        let filtered: Vec<serde_yaml::Value> = existing_rules
            .into_iter()
            .filter(|rule| {
                rule.as_str()
                    .map(|s| !delete_set.contains(s))
                    .unwrap_or(true)
            })
            .collect();

        let mut final_rules = prepend;
        final_rules.extend(filtered);
        final_rules.extend(append);

        map.insert(rules_key, serde_yaml::Value::Sequence(final_rules));
    }

    serde_yaml::to_string(&config_val).unwrap_or_else(|_| yaml.to_string())
}

async fn read_mihomo_overrides() -> String {
    let path = crate::utils::dirs::controled_mihomo_config_path();
    let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
    if content.is_empty() {
        return content;
    }
    if let Ok(mut val) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
        if let serde_yaml::Value::Mapping(ref mut map) = val {
            let before = map.len();
            map.retain(|_, v| !v.is_null());
            if map.len() != before {
                log::info!(
                    "[read_mihomo_overrides] cleaned {} stale null entries from mihomo.yaml",
                    before - map.len()
                );
                let clean = serde_yaml::to_string(&val).unwrap_or_default();
                let _ = tokio::fs::write(&path, &clean).await;
                return clean;
            }
        }
    }
    content
}

fn merge_yaml(base: &str, patch: &str) -> String {
    let mut base_val: serde_yaml::Value = if base.is_empty() {
        serde_yaml::Value::Mapping(Default::default())
    } else {
        serde_yaml::from_str(base).unwrap_or(serde_yaml::Value::Mapping(Default::default()))
    };

    if !patch.is_empty() {
        if let Ok(patch_val) = serde_yaml::from_str::<serde_yaml::Value>(patch) {
            deep_merge_yaml(&mut base_val, patch_val);
        }
    }

    serde_yaml::to_string(&base_val).unwrap_or_default()
}

fn deep_merge_yaml(base: &mut serde_yaml::Value, patch: serde_yaml::Value) {
    if let (serde_yaml::Value::Mapping(ref mut base_map), serde_yaml::Value::Mapping(patch_map)) =
        (base, patch)
    {
        for (k, v) in patch_map {
            if v.is_null() {
                continue;
            }
            if v.is_mapping() {
                if let Some(existing) = base_map.get_mut(&k) {
                    if existing.is_mapping() {
                        deep_merge_yaml(existing, v);
                        continue;
                    }
                }
            }
            base_map.insert(k, v);
        }
    }
}

fn ensure_external_controller_in_yaml(
    yaml: &str,
    preferred_addr: Option<&str>,
) -> (String, String) {
    let mut val: serde_yaml::Value = if yaml.is_empty() {
        serde_yaml::Value::Mapping(Default::default())
    } else {
        serde_yaml::from_str(yaml).unwrap_or(serde_yaml::Value::Mapping(Default::default()))
    };

    let existing_addr = val
        .get("external-controller")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let addr = match existing_addr {
        Some(a) => a,
        None => {
            let a = if let Some(p) = preferred_addr.filter(|s| !s.is_empty()) {
                p.trim_start_matches("http://")
                    .trim_start_matches("https://")
                    .to_string()
            } else {
                let port = (9090u16..9190)
                    .find(|p| std::net::TcpListener::bind(("127.0.0.1", *p)).is_ok())
                    .unwrap_or(9090);
                format!("127.0.0.1:{port}")
            };
            if let serde_yaml::Value::Mapping(ref mut map) = val {
                map.insert(
                    serde_yaml::Value::String("external-controller".into()),
                    serde_yaml::Value::String(a.clone()),
                );
            }
            a
        }
    };

    let url = if addr.starts_with("http") {
        addr
    } else if addr.starts_with(':') {
        format!("http://127.0.0.1{addr}")
    } else {
        format!("http://{addr}")
    };

    (serde_yaml::to_string(&val).unwrap_or_default(), url)
}

pub async fn rebuild_config() -> Result<String> {
    let _lock = REBUILD_LOCK.lock().await;

    let profile_id = read_current_profile_id().await;
    let profile_yaml = read_current_profile_yaml().await;
    let overrides_yaml = read_mihomo_overrides().await;
    log::info!(
        "[rebuild_config] profile_yaml length={}, overrides_yaml length={}, profile_id={:?}",
        profile_yaml.len(),
        overrides_yaml.len(),
        profile_id
    );

    let base_merged = merge_yaml(&profile_yaml, &overrides_yaml);
    let merged = if let Some(ref id) = profile_id {
        apply_rule_overrides(&base_merged, id).await
    } else {
        base_merged
    };

    let running_url = CONTROLLER_URL.lock().clone();
    let preferred_addr = if running_url.is_empty() {
        None
    } else {
        Some(running_url.as_str())
    };

    let (final_yaml, url) = ensure_external_controller_in_yaml(&merged, preferred_addr);

    if let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(&final_yaml) {
        let tun_enable = val.get("tun").and_then(|t| t.get("enable"));
        let ext_ctrl = val.get("external-controller");
        let secret = val.get("secret");
        log::info!(
            "[rebuild_config] tun.enable={:?}, external-controller={:?}, secret={:?}, url={url}",
            tun_enable,
            ext_ctrl,
            secret
        );
    }

    let cm = cm()?;
    let config_path = cm
        .get_current_path()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if let Some(parent) = config_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    tokio::fs::write(&config_path, &final_yaml)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    log::info!("[rebuild_config] wrote config to {:?}", config_path);

    Ok(url)
}

pub async fn install_core() -> Result<()> {
    install_core_for_core_type("mihomo").await
}

pub async fn install_core_for_core_type(core: &str) -> Result<()> {
    if core == "system" {
        return Ok(());
    }

    let want_alpha = core == "mihomo-alpha";
    let channel = if want_alpha {
        Channel::Nightly
    } else {
        Channel::Stable
    };

    let vm = vm()?;
    let version = match vm.install_channel(channel).await {
        Ok(v) => v,
        Err(e) => {
            if e.to_string().contains("already installed") {
                let versions = vm
                    .list_installed()
                    .await
                    .map_err(|e2| anyhow::anyhow!("{e2}"))?;
                let selected = versions
                    .into_iter()
                    .find(|v| version_matches_channel(&v.version, want_alpha));
                selected.map(|v| v.version).ok_or_else(|| {
                    anyhow::anyhow!("no installed versions found for selected core channel")
                })?
            } else {
                return Err(anyhow::anyhow!("{e}"));
            }
        }
    };
    vm.set_default(&version)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

pub async fn start_core() -> Result<String> {
    log::info!("[start_core] stopping any existing core...");
    stop_core().await.ok();

    let app_cfg = tokio::fs::read_to_string(crate::utils::dirs::app_config_path())
        .await
        .ok()
        .and_then(|s| serde_yaml::from_str::<serde_yaml::Value>(&s).ok())
        .unwrap_or_default();
    let core_type = app_cfg
        .get("core")
        .and_then(|v| v.as_str())
        .unwrap_or("mihomo");

    let binary = if core_type == "system" {
        let system_path = app_cfg
            .get("systemCorePath")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("system core path is not configured"))?;
        let p = std::path::PathBuf::from(system_path);
        if !p.exists() {
            return Err(anyhow::anyhow!(
                "system core does not exist: {}",
                p.display()
            ));
        }
        p
    } else {
        install_core_for_core_type(core_type).await?;
        let vm = vm()?;
        vm.get_binary_path(None)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?
    };
    log::info!("[start_core] binary: {:?}", binary);

    let url = rebuild_config().await?;
    log::info!("[start_core] rebuild_config returned url={url}");

    save_controller_to_overrides(&url).await;

    let cm = cm()?;
    let config = cm
        .get_current_path()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    log::info!("[start_core] config path: {:?}", config);

    let secret = extract_secret_from_config(&config).await;
    if secret.is_some() {
        log::info!("[start_core] extracted secret from config (non-empty)");
    }

    spawn_mihomo(&binary, &config).await?;
    log::info!("[start_core] mihomo process started");

    *CONTROLLER_URL.lock() = url.clone();
    super::api::init_client(&url, secret)?;
    log::info!("[start_core] API client initialised");

    wait_for_core_ready(&url).await;

    log::info!("[start_core] mihomo ready at {url}");
    Ok(url)
}

async fn extract_secret_from_config(config_path: &std::path::Path) -> Option<String> {
    let content = tokio::fs::read_to_string(config_path).await.ok()?;
    let val: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    val.get("secret")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

async fn save_controller_to_overrides(url: &str) {
    use crate::utils::dirs;
    let addr = url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .to_string();
    let path = dirs::controled_mihomo_config_path();
    let mut val: serde_yaml::Value = if path.exists() {
        tokio::fs::read_to_string(&path)
            .await
            .ok()
            .and_then(|s| serde_yaml::from_str(&s).ok())
            .unwrap_or(serde_yaml::Value::Mapping(Default::default()))
    } else {
        serde_yaml::Value::Mapping(Default::default())
    };
    if let serde_yaml::Value::Mapping(ref mut map) = val {
        map.insert(
            serde_yaml::Value::String("external-controller".into()),
            serde_yaml::Value::String(addr),
        );
    }
    let _ = tokio::fs::write(&path, serde_yaml::to_string(&val).unwrap_or_default()).await;
}

pub async fn stop_core() -> Result<()> {
    #[cfg(windows)]
    if crate::commands::service::service_status().await == Ok("running".to_string()) {
        log::info!("[stop_core] mihomo relies on background service, skipping local stop");
        return Ok(());
    }

    log::info!("[stop_core] stopping mihomo...");
    match stop_mihomo().await {
        Ok(_) => log::info!("[stop_core] mihomo stopped"),
        Err(e) => log::warn!("[stop_core] stop error (may be expected): {e}"),
    }
    Ok(())
}

pub async fn restart_core() -> Result<String> {
    log::info!("[restart_core] restarting...");
    stop_core().await.ok();
    let result = start_core().await;
    match &result {
        Ok(url) => log::info!("[restart_core] restarted successfully at {url}"),
        Err(e) => log::error!("[restart_core] failed: {e}"),
    }
    result
}

pub fn controller_url() -> String {
    CONTROLLER_URL.lock().clone()
}

pub fn set_controller_url(url: String) {
    *CONTROLLER_URL.lock() = url;
}

pub async fn core_installed() -> bool {
    match vm() {
        Ok(vm) => vm.get_binary_path(None).await.is_ok(),
        Err(_) => false,
    }
}

pub async fn get_installed_version() -> Result<String> {
    let vm = vm()?;
    let versions = vm
        .list_installed()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    versions
        .into_iter()
        .next()
        .map(|v| v.version)
        .ok_or_else(|| anyhow::anyhow!("no installed versions"))
}

async fn wait_for_core_ready(url: &str) {
    let version_url = format!("{url}/version");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();
    for _ in 0..15 {
        if client.get(&version_url).send().await.is_ok() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    log::warn!("mihomo did not become ready within 3 seconds");
}
