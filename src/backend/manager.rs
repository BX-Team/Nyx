use anyhow::Result;
use mihomo_rs::{Channel, ConfigManager, VersionManager};
use once_cell::sync::Lazy;
use tokio::sync::Mutex as AsyncMutex;

static REBUILD_LOCK: Lazy<AsyncMutex<()>> = Lazy::new(|| AsyncMutex::new(()));

fn vm() -> Result<VersionManager> {
    VersionManager::with_home(crate::backend::dirs::data_dir()).map_err(|e| anyhow::anyhow!("{e}"))
}

fn cm() -> Result<ConfigManager> {
    ConfigManager::with_home(crate::backend::dirs::data_dir()).map_err(|e| anyhow::anyhow!("{e}"))
}

fn version_matches_channel(version: &str, want_alpha: bool) -> bool {
    let lower = version.to_ascii_lowercase();
    let is_alpha = lower.contains("alpha")
        || lower.contains("preview")
        || lower.contains("pre")
        || lower.contains("nightly");
    if want_alpha { is_alpha } else { !is_alpha }
}

async fn read_current_profile_id() -> Option<String> {
    let meta_path = crate::backend::dirs::profile_config_path();
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
    use crate::backend::dirs;

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
    let rule_path = crate::backend::dirs::rule_path(profile_id);
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
    let path = crate::backend::dirs::controled_mihomo_config_path();
    let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
    if content.is_empty() {
        return content;
    }
    if let Ok(mut val) = serde_yaml::from_str::<serde_yaml::Value>(&content)
        && let serde_yaml::Value::Mapping(ref mut map) = val
    {
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
    content
}

fn merge_yaml(base: &str, patch: &str) -> String {
    let mut base_val: serde_yaml::Value = if base.is_empty() {
        serde_yaml::Value::Mapping(Default::default())
    } else {
        serde_yaml::from_str(base).unwrap_or(serde_yaml::Value::Mapping(Default::default()))
    };

    if !patch.is_empty()
        && let Ok(patch_val) = serde_yaml::from_str::<serde_yaml::Value>(patch)
    {
        deep_merge_yaml(&mut base_val, patch_val);
    }

    serde_yaml::to_string(&base_val).unwrap_or_default()
}

/// Layers the app's override onto a section. With `force` the app wins; else a
/// non-empty profile section is kept. `tun` always keeps its `enable` key.
fn apply_section_policy(
    profile: &serde_yaml::Value,
    overrides: &mut serde_yaml::Value,
    section: &str,
    force: bool,
) {
    if force {
        return;
    }
    let profile_has = profile
        .get(section)
        .and_then(|v| v.as_mapping())
        .map(|m| !m.is_empty())
        .unwrap_or(false);
    if !profile_has {
        return;
    }
    let serde_yaml::Value::Mapping(map) = overrides else {
        return;
    };
    let key = serde_yaml::Value::String(section.to_string());
    let kept_enable = (section == "tun")
        .then(|| map.get(&key).and_then(|t| t.get("enable")).cloned())
        .flatten();
    map.remove(&key);
    if let Some(enable) = kept_enable {
        let mut tun = serde_yaml::Mapping::new();
        tun.insert(serde_yaml::Value::String("enable".into()), enable);
        map.insert(key, serde_yaml::Value::Mapping(tun));
    }
}

fn deep_merge_yaml(base: &mut serde_yaml::Value, patch: serde_yaml::Value) {
    if let (serde_yaml::Value::Mapping(base_map), serde_yaml::Value::Mapping(patch_map)) =
        (base, patch)
    {
        for (k, v) in patch_map {
            if v.is_null() {
                continue;
            }
            if v.is_mapping()
                && let Some(existing) = base_map.get_mut(&k)
                && existing.is_mapping()
            {
                deep_merge_yaml(existing, v);
                continue;
            }
            base_map.insert(k, v);
        }
    }
}

/// Fixed, so the app and the core agree across restarts without persisting it.
const DEFAULT_CONTROLLER_PORT: u16 = 9097;

fn free_controller_port() -> u16 {
    let taken = |p: u16| std::net::TcpListener::bind(("127.0.0.1", p)).is_err();
    if !taken(DEFAULT_CONTROLLER_PORT) {
        return DEFAULT_CONTROLLER_PORT;
    }
    log::warn!("[core] port {DEFAULT_CONTROLLER_PORT} is busy, picking another");
    (DEFAULT_CONTROLLER_PORT + 1..DEFAULT_CONTROLLER_PORT + 100)
        .find(|p| !taken(*p))
        .unwrap_or(DEFAULT_CONTROLLER_PORT)
}

fn ensure_external_controller_in_yaml(yaml: &str) -> (String, String) {
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
            let a = format!("127.0.0.1:{}", free_controller_port());
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

    let effective_overrides = {
        let profile_val: serde_yaml::Value =
            serde_yaml::from_str(&profile_yaml).unwrap_or(serde_yaml::Value::Null);
        let mut overrides_val: serde_yaml::Value = serde_yaml::from_str(&overrides_yaml)
            .unwrap_or(serde_yaml::Value::Mapping(Default::default()));
        use crate::backend::config::app_config_bool;
        apply_section_policy(
            &profile_val,
            &mut overrides_val,
            "dns",
            app_config_bool("controlDns"),
        );
        apply_section_policy(
            &profile_val,
            &mut overrides_val,
            "sniffer",
            app_config_bool("controlSniff"),
        );
        apply_section_policy(
            &profile_val,
            &mut overrides_val,
            "tun",
            app_config_bool("controlTun"),
        );
        serde_yaml::to_string(&overrides_val).unwrap_or(overrides_yaml)
    };

    let base_merged = merge_yaml(&profile_yaml, &effective_overrides);
    let merged = if let Some(ref id) = profile_id {
        apply_rule_overrides(&base_merged, id).await
    } else {
        base_merged
    };

    let (final_yaml, url) = ensure_external_controller_in_yaml(&merged);

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

fn core_binary_name() -> &'static str {
    if cfg!(windows) {
        "mihomo.exe"
    } else {
        "mihomo"
    }
}

pub async fn ensure_core_installed(core: &str) -> Result<()> {
    if core == "system" {
        return Ok(());
    }
    let want_alpha = core == "mihomo-alpha";
    let vm = vm()?;
    let installed = vm.list_installed().await.unwrap_or_default();
    for v in installed
        .into_iter()
        .filter(|v| version_matches_channel(&v.version, want_alpha))
    {
        if v.path.join(core_binary_name()).exists() {
            vm.set_default(&v.version)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            return Ok(());
        }
        log::warn!("[core] removing broken install {}", v.version);
        let _ = tokio::fs::remove_dir_all(&v.path).await;
    }
    install_core_for_core_type(core).await
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
                let selected = versions.into_iter().find(|v| {
                    version_matches_channel(&v.version, want_alpha)
                        && v.path.join(core_binary_name()).exists()
                });
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
