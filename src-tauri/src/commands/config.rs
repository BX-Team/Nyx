use base64::Engine as _;
use mihomo_rs::ConfigManager;
use serde_json::Value;
use std::fs;


fn mihomo_config_manager() -> anyhow::Result<ConfigManager> {
    ConfigManager::with_home(crate::utils::dirs::data_dir()).map_err(|e| anyhow::anyhow!("{e}"))
}

fn read_yaml_as_json(path: &std::path::Path) -> anyhow::Result<Value> {
    let text = fs::read_to_string(path)?;
    let value: Value = serde_yaml::from_str(&text)?;
    Ok(value)
}

fn write_json_as_yaml(path: &std::path::Path, value: &Value) -> anyhow::Result<()> {
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
                let entry = base_map.entry(k).or_insert_with(|| Value::Object(Default::default()));
                merge_patch(entry, v);
            } else {
                base_map.insert(k, v);
            }
        }
    }
}


#[tauri::command]
pub async fn get_app_config(_force: Option<bool>) -> Result<Value, String> {
    let path = crate::utils::dirs::app_config_path();
    if !path.exists() {
        return Ok(Value::Object(Default::default()));
    }
    read_yaml_as_json(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn patch_app_config(config: Value) -> Result<(), String> {
    let path = crate::utils::dirs::app_config_path();
    let mut base = if path.exists() {
        read_yaml_as_json(&path).unwrap_or(Value::Object(Default::default()))
    } else {
        Value::Object(Default::default())
    };
    merge_patch(&mut base, config);
    write_json_as_yaml(&path, &base).map_err(|e| e.to_string())
}


#[tauri::command]
pub async fn get_controled_mihomo_config(_force: Option<bool>) -> Result<Value, String> {
    let mgr: ConfigManager = mihomo_config_manager().map_err(|e| e.to_string())?;
    let path = mgr.get_current_path().await.map_err(|e| e.to_string())?;
    log::debug!("[get_controled_mihomo_config] reading from {:?}, exists={}", path, path.exists());
    if !path.exists() {
        return Ok(Value::Object(Default::default()));
    }
    let result = read_yaml_as_json(&path).map_err(|e| e.to_string());
    if let Ok(ref val) = result {
        let tun_enable = val.get("tun").and_then(|t| t.get("enable"));
        log::debug!("[get_controled_mihomo_config] tun.enable={:?}", tun_enable);
    }
    result
}

#[tauri::command]
pub async fn patch_controled_mihomo_config(
    app: tauri::AppHandle,
    config: Value,
) -> Result<(), String> {
    log::info!("[patch_controled_mihomo_config] received patch: {}", serde_json::to_string(&config).unwrap_or_default());

    let overrides_path = crate::utils::dirs::controled_mihomo_config_path();
    let mut base = if overrides_path.exists() {
        read_yaml_as_json(&overrides_path).unwrap_or(Value::Object(Default::default()))
    } else {
        Value::Object(Default::default())
    };
    merge_patch(&mut base, config.clone());
    strip_nulls(&mut base); 
    write_json_as_yaml(&overrides_path, &base).map_err(|e| e.to_string())?;
    log::info!("[patch_controled_mihomo_config] wrote overrides to {:?}", overrides_path);

    let mgr: ConfigManager = mihomo_config_manager().map_err(|e| e.to_string())?;
    let config_path = mgr.get_current_path().await.map_err(|e| e.to_string())?;
    if config_path.exists() {
        let mut running = read_yaml_as_json(&config_path)
            .unwrap_or(Value::Object(Default::default()));
        merge_patch(&mut running, config.clone());
        write_json_as_yaml(&config_path, &running).map_err(|e| e.to_string())?;
    }

    let patch_url = format!("{}/configs", crate::core::manager::controller_url());
    let _ = reqwest::Client::new()
        .patch(&patch_url)
        .json(&config)
        .send()
        .await;

    use tauri::Emitter;
    let _ = app.emit("controled-mihomo-config-updated", ());
    Ok(())
}


#[tauri::command]
pub async fn get_profile_config(_force: Option<bool>) -> Result<Value, String> {
    let path = crate::utils::dirs::profile_config_path();
    if !path.exists() {
        return Ok(serde_json::json!({ "current": null, "items": [] }));
    }
    read_yaml_as_json(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_profile_config(config: Value) -> Result<(), String> {
    let path = crate::utils::dirs::profile_config_path();
    write_json_as_yaml(&path, &config).map_err(|e| e.to_string())
}


#[tauri::command]
pub async fn get_current_profile_item() -> Result<Value, String> {
    let cfg = get_profile_config(None).await?;
    let current_id = cfg["current"].as_str().unwrap_or("").to_string();
    if current_id.is_empty() {
        return Ok(Value::Null);
    }
    get_profile_item(current_id).await
}

#[tauri::command]
pub async fn get_profile_item(id: String) -> Result<Value, String> {
    let cfg = get_profile_config(None).await?;
    if let Some(items) = cfg["items"].as_array() {
        for item in items {
            if item["id"].as_str() == Some(&id) {
                return Ok(item.clone());
            }
        }
    }
    Err(format!("profile '{id}' not found"))
}

#[tauri::command]
pub async fn get_profile_str(id: String) -> Result<String, String> {
    let path = crate::utils::dirs::profile_path(&id);
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_profile_str(id: String, str: String) -> Result<(), String> {
    let path = crate::utils::dirs::profile_path(&id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, str).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_current_profile_str() -> Result<String, String> {
    let cfg = get_profile_config(None).await?;
    let id = cfg["current"].as_str().unwrap_or("").to_string();
    if id.is_empty() {
        return Err("no current profile set".to_string());
    }
    get_profile_str(id).await
}

#[tauri::command]
pub async fn get_raw_profile_str() -> Result<String, String> {
    get_current_profile_str().await
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


#[tauri::command]
pub async fn add_profile_item(app: tauri::AppHandle, item: Value) -> Result<(), String> {
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
                .unwrap_or_else(|| format!("clash-meta/mihomo/Nyx-v{}", app.package_info().version));
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

            let body = resp.text().await.map_err(|e| e.to_string())?;
            let profile_path = crate::utils::dirs::profile_path(&id);
            if let Some(parent) = profile_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(&profile_path, body).map_err(|e| e.to_string())?;
        }
    } else {
        if let Some(content) = item["file"].as_str() {
            let profile_path = crate::utils::dirs::profile_path(&id);
            if let Some(parent) = profile_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(&profile_path, content).map_err(|e| e.to_string())?;
        }
        if let Some(obj) = meta.as_object_mut() {
            obj.remove("file");
        }
    }

    let config_path = crate::utils::dirs::profile_config_path();
    let mut cfg = get_profile_config(None).await?;

    let items = cfg["items"].as_array_mut().ok_or("invalid profile config")?;
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
        if cfg["current"].as_str().map(|s| s.is_empty()).unwrap_or(true)
            || cfg["current"].is_null()
        {
            cfg["current"] = Value::String(id.clone());
            became_current = true;
        }
    }

    write_json_as_yaml(&config_path, &cfg).map_err(|e| e.to_string())?;

    use tauri::Emitter;
    let _ = app.emit("profile-config-updated", ());

    if became_current || existing_idx.is_some() {
        if let Ok(current_id) = get_profile_config(None).await.and_then(|c| {
            c["current"]
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| "no current".to_string())
        }) {
            if current_id == id {
                let _ = crate::core::manager::rebuild_config().await;
                let mgr = mihomo_config_manager().map_err(|e| e.to_string())?;
                let config_path = mgr.get_current_path().await.map_err(|e| e.to_string())?;
                let path_str = config_path.to_string_lossy().replace('\\', "/");
                let reload_url = format!(
                    "{}/configs?force=false",
                    crate::core::manager::controller_url()
                );
                let _ = reqwest::Client::new()
                    .put(&reload_url)
                    .json(&serde_json::json!({ "path": path_str }))
                    .send()
                    .await;
                let _ = app.emit("controled-mihomo-config-updated", ());
                let _ = app.emit("groups-updated", ());
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn update_profile_item(item: Value) -> Result<(), String> {
    let id = item["id"].as_str().ok_or("item missing id")?.to_string();
    let path = crate::utils::dirs::profile_config_path();
    let mut cfg = get_profile_config(None).await?;
    let items = cfg["items"].as_array_mut().ok_or("invalid profile config")?;
    for existing in items.iter_mut() {
        if existing["id"].as_str() == Some(&id) {
            *existing = item;
            return write_json_as_yaml(&path, &cfg).map_err(|e| e.to_string());
        }
    }
    Err(format!("profile '{id}' not found"))
}

#[tauri::command]
pub async fn remove_profile_item(id: String) -> Result<(), String> {
    let path = crate::utils::dirs::profile_config_path();
    let mut cfg = get_profile_config(None).await?;
    let items = cfg["items"].as_array_mut().ok_or("invalid profile config")?;
    items.retain(|item| item["id"].as_str() != Some(&id));
    let profile_path = crate::utils::dirs::profile_path(&id);
    let _ = fs::remove_file(&profile_path);
    write_json_as_yaml(&path, &cfg).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reload_current_profile(app: tauri::AppHandle) -> Result<(), String> {
    let cfg = get_profile_config(None).await?;
    let current_id = cfg["current"].as_str().unwrap_or("").to_string();
    if current_id.is_empty() {
        return Err("no current profile set".to_string());
    }

    crate::core::manager::rebuild_config()
        .await
        .map_err(|e| e.to_string())?;

    let mgr: ConfigManager = mihomo_config_manager().map_err(|e| e.to_string())?;
    let config_path = mgr.get_current_path().await.map_err(|e| e.to_string())?;
    let path_str = config_path.to_string_lossy().replace('\\', "/");
    let reload_url = format!("{}/configs?force=false", crate::core::manager::controller_url());
    let _ = reqwest::Client::new()
        .put(&reload_url)
        .json(&serde_json::json!({ "path": path_str }))
        .send()
        .await;

    use tauri::Emitter;
    let _ = app.emit("controled-mihomo-config-updated", ());
    let _ = app.emit("groups-updated", ());
    Ok(())
}

#[tauri::command]
pub async fn change_current_profile(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let path = crate::utils::dirs::profile_config_path();
    let mut cfg = get_profile_config(None).await?;
    let exists = cfg["items"]
        .as_array()
        .map(|items| items.iter().any(|i| i["id"].as_str() == Some(&id)))
        .unwrap_or(false);
    if !exists {
        return Err(format!("profile '{id}' not found"));
    }
    cfg["current"] = Value::String(id);
    write_json_as_yaml(&path, &cfg).map_err(|e| e.to_string())?;

    crate::core::manager::rebuild_config()
        .await
        .map_err(|e| e.to_string())?;

    let mgr: ConfigManager = mihomo_config_manager().map_err(|e| e.to_string())?;
    let config_path = mgr.get_current_path().await.map_err(|e| e.to_string())?;
    let path_str = config_path.to_string_lossy().replace('\\', "/");
    let reload_url = format!("{}/configs?force=false", crate::core::manager::controller_url());
    let body = serde_json::json!({ "path": path_str });
    let result = reqwest::Client::new()
        .put(&reload_url)
        .json(&body)
        .send()
        .await;
    if let Err(e) = result {
        log::warn!("Failed to hot-reload mihomo after profile change: {e}");
    }

    use tauri::Emitter;
    let _ = app.emit("controled-mihomo-config-updated", ());
    let _ = app.emit("profile-config-updated", ());
    let _ = app.emit("groups-updated", ());
    Ok(())
}


#[tauri::command]
pub async fn get_runtime_config() -> Result<Value, String> {
    crate::core::api::get_config().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_runtime_config_str() -> Result<String, String> {
    let val = crate::core::api::get_config().await.map_err(|e| e.to_string())?;
    serde_yaml::to_string(&val).map_err(|e| e.to_string())
}


#[tauri::command]
pub async fn get_file_str(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_file_str(path: String, str: String) -> Result<(), String> {
    fs::write(&path, str).map_err(|e| e.to_string())
}


#[tauri::command]
pub async fn get_rule_str(id: String) -> Result<String, String> {
    let path = crate::utils::dirs::rule_path(&id);
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_rule_str(id: String, str: String) -> Result<(), String> {
    let path = crate::utils::dirs::rule_path(&id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&path, str).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn convert_mrs_ruleset(path: String, behavior: String) -> Result<(), String> {
    let _ = (path, behavior);
    Ok(())
}
