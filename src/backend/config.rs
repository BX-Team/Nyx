use anyhow::Result;
use base64::Engine as _;
use mihomo_rs::ConfigManager;
use serde_json::Value;
use std::fs;

use crate::backend::dirs;

fn mihomo_config_manager() -> Result<ConfigManager> {
    ConfigManager::with_home(dirs::data_dir()).map_err(|e| anyhow::anyhow!("{e}"))
}

fn read_yaml_as_json(path: &std::path::Path) -> Result<Value> {
    let text = fs::read_to_string(path)?;
    let value: Value = serde_yaml::from_str(&text)?;
    Ok(value)
}

fn write_json_as_yaml(path: &std::path::Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_yaml::to_string(value)?;
    fs::write(path, text)?;
    Ok(())
}

fn strip_nulls(val: &mut Value) {
    if let Value::Object(map) = val {
        map.retain(|_, v| !v.is_null());
        for v in map.values_mut() {
            strip_nulls(v);
        }
    }
}

fn merge_patch(base: &mut Value, patch: Value) {
    if let (Value::Object(base_map), Value::Object(patch_map)) = (base, patch) {
        for (k, v) in patch_map {
            if v.is_null() {
                base_map.remove(&k);
            } else if v.is_object() {
                let entry = base_map
                    .entry(k)
                    .or_insert_with(|| Value::Object(Default::default()));
                merge_patch(entry, v);
            } else {
                base_map.insert(k, v);
            }
        }
    }
}

pub async fn get_app_config() -> Result<Value> {
    let path = dirs::app_config_path();
    if !path.exists() {
        return Ok(Value::Object(Default::default()));
    }
    read_yaml_as_json(&path)
}

pub async fn patch_app_config(config: Value) -> Result<()> {
    let path = dirs::app_config_path();
    let mut base = if path.exists() {
        read_yaml_as_json(&path).unwrap_or(Value::Object(Default::default()))
    } else {
        Value::Object(Default::default())
    };
    merge_patch(&mut base, config);
    write_json_as_yaml(&path, &base)
}

/// Reads a top-level boolean flag from the app config synchronously (used at
/// startup before the async config load lands in state).
pub fn app_config_bool(key: &str) -> bool {
    let path = dirs::app_config_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_yaml::from_str::<Value>(&s).ok())
        .and_then(|v| v.get(key).and_then(Value::as_bool))
        .unwrap_or(false)
}

/// Reads the persisted main-window geometry `(x, y, width, height)` from the app
/// config, if present. Sync — called while opening the window.
pub fn load_window_state() -> Option<(f64, f64, f64, f64)> {
    let path = dirs::app_config_path();
    let v: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_yaml::from_str(&s).ok())?;
    let w = v.get("window")?;
    Some((
        w.get("x")?.as_f64()?,
        w.get("y")?.as_f64()?,
        w.get("width")?.as_f64()?,
        w.get("height")?.as_f64()?,
    ))
}

/// Persists the main-window geometry into the app config under `window`. Sync —
/// called from the window close/hide path (no gpui borrow held).
pub fn save_window_state(x: f64, y: f64, width: f64, height: f64) {
    let path = dirs::app_config_path();
    let mut v: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_yaml::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = v.as_object_mut() {
        obj.insert(
            "window".to_string(),
            serde_json::json!({ "x": x, "y": y, "width": width, "height": height }),
        );
    }
    if let Ok(s) = serde_yaml::to_string(&v) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, s);
    }
}

pub async fn get_profile_config() -> Result<Value> {
    let path = dirs::profile_config_path();
    if !path.exists() {
        return Ok(Value::Object(Default::default()));
    }
    read_yaml_as_json(&path)
}

pub async fn get_controled_mihomo_config() -> Result<Value> {
    let mgr = mihomo_config_manager()?;
    let path = mgr
        .get_current_path()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if !path.exists() {
        return Ok(Value::Object(Default::default()));
    }
    read_yaml_as_json(&path)
}

/// Persist a patch to the controlled mihomo overrides, mirror it into the live
/// runtime config, and PATCH it to the running core. Used by the Home
/// main-switch (TUN toggle) and other live config tweaks.
pub async fn patch_controled_mihomo_config(config: Value) -> Result<()> {
    let overrides_path = dirs::controled_mihomo_config_path();
    let mut base = if overrides_path.exists() {
        read_yaml_as_json(&overrides_path).unwrap_or(Value::Object(Default::default()))
    } else {
        Value::Object(Default::default())
    };
    merge_patch(&mut base, config.clone());
    strip_nulls(&mut base);
    write_json_as_yaml(&overrides_path, &base)?;

    let mgr = mihomo_config_manager()?;
    let config_path = mgr
        .get_current_path()
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if config_path.exists() {
        let mut running =
            read_yaml_as_json(&config_path).unwrap_or(Value::Object(Default::default()));
        merge_patch(&mut running, config.clone());
        write_json_as_yaml(&config_path, &running)?;
    }

    let patch_url = format!("{}/configs", crate::backend::manager::controller_url());
    let _ = reqwest::Client::new()
        .patch(&patch_url)
        .json(&config)
        .send()
        .await;
    Ok(())
}

/// Nyx user-agent for subscription fetches.
fn nyx_user_agent() -> String {
    format!("clash-meta/mihomo/Nyx-v{}", env!("CARGO_PKG_VERSION"))
}

pub async fn set_profile_config(config: Value) -> Result<(), String> {
    let path = dirs::profile_config_path();
    write_json_as_yaml(&path, &config).map_err(|e| e.to_string())
}

async fn profile_config() -> Result<Value, String> {
    get_profile_config().await.map_err(|e| e.to_string())
}

pub async fn get_current_profile_item() -> Result<Value, String> {
    let cfg = profile_config().await?;
    let current_id = cfg["current"].as_str().unwrap_or("").to_string();
    if current_id.is_empty() {
        return Ok(Value::Null);
    }
    get_profile_item(current_id).await
}

pub async fn get_profile_item(id: String) -> Result<Value, String> {
    let cfg = profile_config().await?;
    if let Some(items) = cfg["items"].as_array() {
        for item in items {
            if item["id"].as_str() == Some(&id) {
                return Ok(item.clone());
            }
        }
    }
    Err(format!("profile '{id}' not found"))
}

pub async fn get_profile_str(id: String) -> Result<String, String> {
    let path = dirs::profile_path(&id);
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

pub async fn set_profile_str(id: String, str: String) -> Result<(), String> {
    let path = dirs::profile_path(&id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, str).map_err(|e| e.to_string())
}

pub async fn get_current_profile_str() -> Result<String, String> {
    let cfg = profile_config().await?;
    let id = cfg["current"].as_str().unwrap_or("").to_string();
    if id.is_empty() {
        return Err("no current profile set".to_string());
    }
    get_profile_str(id).await
}

pub async fn get_raw_profile_str() -> Result<String, String> {
    get_current_profile_str().await
}

pub async fn get_rule_str(id: String) -> Result<String, String> {
    let path = dirs::rule_path(&id);
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

pub async fn set_rule_str(id: String, str: String) -> Result<(), String> {
    let path = dirs::rule_path(&id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, str).map_err(|e| e.to_string())
}

fn decode_header_value(value: &str) -> String {
    if let Some(encoded) = value.strip_prefix("base64:") {
        if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded.trim()) {
            if let Ok(s) = String::from_utf8(bytes) {
                return s;
            }
        }
    }
    value.to_string()
}

fn parse_subscription_userinfo(info: &str) -> Value {
    let mut upload: i64 = 0;
    let mut download: i64 = 0;
    let mut total: i64 = 0;
    let mut expire: i64 = 0;
    for part in info.split(';') {
        if let Some((k, v)) = part.trim().split_once('=') {
            match k.trim() {
                "upload" => upload = v.trim().parse().unwrap_or(0),
                "download" => download = v.trim().parse().unwrap_or(0),
                "total" => total = v.trim().parse().unwrap_or(0),
                "expire" => expire = v.trim().parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    serde_json::json!({ "upload": upload, "download": download, "total": total, "expire": expire })
}

/// Imports or refreshes a profile item (remote subscription or local file),
/// updates `profile.yaml`, and hot-reloads the core if the active profile
/// changed. Returns the profile id.
pub async fn add_profile_item(item: Value) -> Result<String, String> {
    let id = match item["id"].as_str().filter(|s| !s.is_empty()) {
        Some(existing_id) => existing_id.to_string(),
        None => chrono::Utc::now().timestamp_millis().to_string(),
    };

    let item_type = item["type"].as_str().unwrap_or("remote");
    let mut meta = item.clone();
    meta["id"] = Value::String(id.clone());
    meta["updated"] = Value::Number(chrono::Utc::now().timestamp_millis().into());

    if item_type == "remote" {
        if let Some(url) = item["url"].as_str().filter(|s| !s.is_empty()) {
            let ua = item["ua"]
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(nyx_user_agent);
            let client = reqwest::Client::builder()
                .user_agent(&ua)
                .build()
                .map_err(|e| e.to_string())?;

            let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
            let headers = resp.headers().clone();

            if let Some(v) = headers.get("subscription-userinfo") {
                if let Ok(s) = v.to_str() {
                    meta["extra"] = parse_subscription_userinfo(s);
                }
            }
            if meta["name"].as_str().map(|s| s.is_empty()).unwrap_or(true) {
                if let Some(v) = headers.get("profile-title") {
                    if let Ok(s) = v.to_str() {
                        meta["name"] = Value::String(decode_header_value(s));
                    }
                }
            }
            if let Some(v) = headers.get("profile-update-interval") {
                if let Ok(s) = v.to_str() {
                    if let Ok(h) = s.trim().parse::<i64>() {
                        meta["interval"] = Value::Number((h * 60).into());
                    }
                }
            }
            if let Some(v) = headers.get("profile-web-page-url") {
                if let Ok(s) = v.to_str() {
                    meta["home"] = Value::String(s.to_string());
                }
            }
            if let Some(v) = headers.get("support-url") {
                if let Ok(s) = v.to_str() {
                    meta["supportUrl"] = Value::String(s.to_string());
                }
            }
            if let Some(v) = headers.get("announce") {
                if let Ok(s) = v.to_str() {
                    meta["announce"] = Value::String(decode_header_value(s));
                }
            }

            let raw_body = resp.text().await.map_err(|e| e.to_string())?;
            let body = crate::backend::proxy_convert::detect_and_convert_subscription(&raw_body);
            let profile_path = dirs::profile_path(&id);
            if let Some(parent) = profile_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(&profile_path, body).map_err(|e| e.to_string())?;
        }
    } else {
        if let Some(content) = item["file"].as_str() {
            let profile_path = dirs::profile_path(&id);
            if let Some(parent) = profile_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(&profile_path, content).map_err(|e| e.to_string())?;
        }
        if let Some(obj) = meta.as_object_mut() {
            obj.remove("file");
        }
    }

    let config_path = dirs::profile_config_path();
    let mut cfg = profile_config().await?;
    let items = cfg["items"]
        .as_array_mut()
        .ok_or("invalid profile config")?;
    let existing_idx = items.iter().position(|i| i["id"].as_str() == Some(&id));

    let mut became_current = false;
    if let Some(idx) = existing_idx {
        if let (Some(existing), Some(new_obj)) = (items[idx].as_object_mut(), meta.as_object()) {
            for (k, v) in new_obj {
                existing.insert(k.clone(), v.clone());
            }
        }
    } else {
        items.push(meta);
        if cfg["current"]
            .as_str()
            .map(|s| s.is_empty())
            .unwrap_or(true)
            || cfg["current"].is_null()
        {
            cfg["current"] = Value::String(id.clone());
            became_current = true;
        }
    }

    write_json_as_yaml(&config_path, &cfg).map_err(|e| e.to_string())?;

    if became_current || existing_idx.is_some() {
        let current_id = profile_config()
            .await
            .ok()
            .and_then(|c| c["current"].as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        if current_id == id {
            reload_core().await;
        }
    }

    Ok(id)
}

/// Rebuilds the merged runtime config and asks the running core to hot-reload
/// it. Best-effort: logs but does not fail on reload errors.
async fn reload_core() {
    if let Err(e) = crate::backend::manager::rebuild_config().await {
        log::warn!("[reload_core] rebuild_config failed: {e}");
        return;
    }
    let Ok(mgr) = mihomo_config_manager() else {
        return;
    };
    let Ok(config_path) = mgr.get_current_path().await else {
        return;
    };
    let path_str = config_path.to_string_lossy().replace('\\', "/");
    let reload_url = format!(
        "{}/configs?force=false",
        crate::backend::manager::controller_url()
    );
    if let Err(e) = reqwest::Client::new()
        .put(&reload_url)
        .json(&serde_json::json!({ "path": path_str }))
        .send()
        .await
    {
        log::warn!("[reload_core] hot-reload request failed: {e}");
    }
}

pub async fn update_profile_item(item: Value) -> Result<(), String> {
    let id = item["id"].as_str().ok_or("item missing id")?.to_string();
    let path = dirs::profile_config_path();
    let mut cfg = profile_config().await?;
    let items = cfg["items"]
        .as_array_mut()
        .ok_or("invalid profile config")?;
    for existing in items.iter_mut() {
        if existing["id"].as_str() == Some(&id) {
            *existing = item;
            return write_json_as_yaml(&path, &cfg).map_err(|e| e.to_string());
        }
    }
    Err(format!("profile '{id}' not found"))
}

pub async fn remove_profile_item(id: String) -> Result<(), String> {
    let path = dirs::profile_config_path();
    let mut cfg = profile_config().await?;
    let items = cfg["items"]
        .as_array_mut()
        .ok_or("invalid profile config")?;
    items.retain(|item| item["id"].as_str() != Some(&id));
    let profile_path = dirs::profile_path(&id);
    let _ = fs::remove_file(&profile_path);
    write_json_as_yaml(&path, &cfg).map_err(|e| e.to_string())
}

pub async fn reload_current_profile() -> Result<(), String> {
    let cfg = profile_config().await?;
    let current_id = cfg["current"].as_str().unwrap_or("").to_string();
    if current_id.is_empty() {
        return Err("no current profile set".to_string());
    }
    reload_core().await;
    Ok(())
}

pub async fn change_current_profile(id: String) -> Result<(), String> {
    let path = dirs::profile_config_path();
    let mut cfg = profile_config().await?;
    let exists = cfg["items"]
        .as_array()
        .map(|items| items.iter().any(|i| i["id"].as_str() == Some(&id)))
        .unwrap_or(false);
    if !exists {
        return Err(format!("profile '{id}' not found"));
    }
    cfg["current"] = Value::String(id);
    write_json_as_yaml(&path, &cfg).map_err(|e| e.to_string())?;
    reload_core().await;
    Ok(())
}

pub async fn get_runtime_config() -> Result<Value, String> {
    crate::backend::api::get_config()
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_runtime_config_str() -> Result<String, String> {
    let val = crate::backend::api::get_config()
        .await
        .map_err(|e| e.to_string())?;
    serde_yaml::to_string(&val).map_err(|e| e.to_string())
}

async fn resolve_provider_path(path: &str) -> std::path::PathBuf {
    use std::path::{Path, PathBuf};
    let clean = path.trim_start_matches("./").trim_start_matches(".\\");
    if Path::new(clean).is_absolute() {
        return PathBuf::from(clean);
    }
    let config_dir = if let Ok(cm) = mihomo_config_manager() {
        if let Ok(config_path) = cm.get_current_path().await {
            config_path
                .parent()
                .map(PathBuf::from)
                .unwrap_or_else(dirs::data_dir)
        } else {
            dirs::data_dir()
        }
    } else {
        dirs::data_dir()
    };
    config_dir.join(clean)
}

pub async fn get_file_str(path: String) -> Result<String, String> {
    let full = resolve_provider_path(&path).await;
    fs::read_to_string(&full).map_err(|e| e.to_string())
}

pub async fn set_file_str(path: String, str: String) -> Result<(), String> {
    let full = resolve_provider_path(&path).await;
    fs::write(&full, str).map_err(|e| e.to_string())
}

/// Reads the contents of a rule/proxy provider for the Resources viewer.
/// Resolves the on-disk path from the running mihomo config (`path`, else
/// `<rules|proxies>/<md5(url)>`), converting `.mrs` rulesets to text on the fly
/// and surfacing inline providers' embedded payloads. Mirrors the old `Viewer`.
pub async fn read_provider_content(name: String, is_rule: bool) -> Result<String, String> {
    use md5::{Digest, Md5};

    let cfg = get_controled_mihomo_config()
        .await
        .map_err(|e| e.to_string())?;
    let key = if is_rule {
        "rule-providers"
    } else {
        "proxy-providers"
    };
    let provider = cfg
        .get(key)
        .and_then(|p| p.get(&name))
        .ok_or_else(|| format!("provider '{name}' not found in runtime config"))?;

    let vehicle = provider
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    // Inline providers carry their payload directly in the config.
    if vehicle == "inline" {
        if let Some(payload) = provider.get("payload") {
            let doc = if is_rule {
                serde_json::json!({ "rules": payload })
            } else {
                serde_json::json!({ "proxies": payload })
            };
            return serde_yaml::to_string(&doc).map_err(|e| e.to_string());
        }
    }

    let behavior = provider
        .get("behavior")
        .and_then(|v| v.as_str())
        .unwrap_or("domain")
        .to_string();
    let format = provider
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let path = if let Some(p) = provider.get("path").and_then(|v| v.as_str()) {
        p.to_string()
    } else if let Some(url) = provider.get("url").and_then(|v| v.as_str()) {
        let mut hasher = Md5::new();
        hasher.update(url.as_bytes());
        let hex = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let sub = if is_rule { "rules" } else { "proxies" };
        format!("{sub}/{hex}")
    } else {
        return Err("provider has neither a path nor a url".into());
    };

    let is_mrs = format.eq_ignore_ascii_case("mrs")
        || format == "MrsRule"
        || path.to_ascii_lowercase().ends_with(".mrs");
    if is_mrs {
        convert_mrs_ruleset(path, behavior).await
    } else {
        get_file_str(path).await
    }
}

/// Wipes the entire app data directory (config, profiles, core, geo data) after
/// best-effort stopping the core. The caller is expected to relaunch the app so
/// it re-initialises from scratch. Mirrors the old `reset_app_config` command.
pub async fn reset_app_config() -> Result<(), String> {
    let _ = crate::backend::manager::stop_core().await;
    let data_dir = dirs::data_dir();
    if data_dir.exists() {
        fs::remove_dir_all(&data_dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn convert_mrs_ruleset(path: String, behavior: String) -> Result<String, String> {
    let vm = mihomo_rs::VersionManager::with_home(dirs::data_dir()).map_err(|e| e.to_string())?;
    let core_path = vm.get_binary_path(None).await.map_err(|e| e.to_string())?;
    let full_path = resolve_provider_path(&path).await;

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let temp_path =
        std::env::temp_dir().join(format!("mrs-convert-{}-{}.txt", std::process::id(), nanos));

    let mut cmd = tokio::process::Command::new(&core_path);
    cmd.arg("convert-ruleset")
        .arg(&behavior)
        .arg("mrs")
        .arg(&full_path)
        .arg(&temp_path);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000);
    let output = cmd.output().await.map_err(|e| e.to_string())?;

    if !output.status.success() {
        let _ = fs::remove_file(&temp_path);
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    let content = fs::read_to_string(&temp_path).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(&temp_path);
    Ok(content)
}
