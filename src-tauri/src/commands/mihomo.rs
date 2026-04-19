use serde_json::Value;


#[tauri::command]
pub async fn mihomo_version() -> Result<String, String> {
    crate::core::api::get_version().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mihomo_installed_version() -> Result<String, String> {
    crate::core::manager::get_installed_version()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mihomo_config() -> Result<Value, String> {
    crate::core::api::get_config().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn patch_mihomo_config(patch: Value) -> Result<(), String> {
    crate::core::api::patch_config(patch).await.map_err(|e| e.to_string())
}


#[tauri::command]
pub async fn mihomo_rules() -> Result<Value, String> {
    crate::core::api::get_rules().await.map_err(|e| e.to_string())
}


#[tauri::command]
pub async fn mihomo_proxies() -> Result<Value, String> {
    let map = crate::core::api::get_raw_proxies_map()
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(map).unwrap_or(Value::Null))
}

#[tauri::command]
pub async fn mihomo_groups() -> Result<Value, String> {
    let proxies = crate::core::api::get_raw_proxies_map()
        .await
        .map_err(|e| e.to_string())?;

    const GROUP_TYPES: &[&str] = &["Selector", "URLTest", "Fallback", "LoadBalance", "Relay"];

    let mut groups: Vec<Value> = proxies
        .values()
        .filter(|p| {
            let is_group = p["type"]
                .as_str()
                .map(|t| GROUP_TYPES.contains(&t))
                .unwrap_or(false);
            let is_global = p["name"].as_str() == Some("GLOBAL");
            let is_hidden = p["hidden"].as_bool().unwrap_or(false);
            is_group && !is_global && !is_hidden
        })
        .map(|group_info| {
            let resolved_all: Vec<Value> = group_info["all"]
                .as_array()
                .map(|names| {
                    names
                        .iter()
                        .filter_map(|n| n.as_str())
                        .filter_map(|n| proxies.get(n).cloned())
                        .collect()
                })
                .unwrap_or_default();

            let mut group = group_info.clone();
            group["all"] = Value::Array(resolved_all);
            group
        })
        .collect();

    groups.sort_by(|a, b| {
        a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
    });

    Ok(Value::Array(groups))
}

#[tauri::command]
pub async fn mihomo_change_proxy(group: String, proxy: String) -> Result<(), String> {
    let encoded_group = group.replace(' ', "%20");
    let url = format!(
        "{}/proxies/{}",
        crate::core::manager::controller_url(),
        encoded_group
    );
    reqwest::Client::new()
        .put(&url)
        .json(&serde_json::json!({ "name": proxy }))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    save_selection(&group, &proxy);
    Ok(())
}

fn save_selection(group: &str, proxy: &str) {
    let path = crate::utils::dirs::selections_path();
    let mut map: serde_yaml::Mapping = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_yaml::from_str::<serde_yaml::Mapping>(&s).ok())
        .unwrap_or_default();
    map.insert(
        serde_yaml::Value::String(group.to_string()),
        serde_yaml::Value::String(proxy.to_string()),
    );
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_yaml::to_string(&map) {
        let _ = std::fs::write(&path, text);
    }
}

pub async fn restore_proxy_selections() {
    let path = crate::utils::dirs::selections_path();
    let Ok(text) = std::fs::read_to_string(&path) else { return };
    let Ok(map) = serde_yaml::from_str::<serde_yaml::Mapping>(&text) else { return };
    let base_url = crate::core::manager::controller_url();
    if base_url.is_empty() {
        return;
    }
    let client = reqwest::Client::new();
    for (k, v) in map.iter() {
        let (Some(group), Some(proxy)) = (k.as_str(), v.as_str()) else { continue };
        let encoded_group = group.replace(' ', "%20");
        let url = format!("{}/proxies/{}", base_url, encoded_group);
        match client
            .put(&url)
            .json(&serde_json::json!({ "name": proxy }))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => {
                log::info!("[restore_proxy_selections] {} -> {}", group, proxy);
            }
            Ok(r) => {
                log::warn!("[restore_proxy_selections] {} -> {} returned {}", group, proxy, r.status());
            }
            Err(e) => {
                log::warn!("[restore_proxy_selections] {} -> {} failed: {e}", group, proxy);
            }
        }
    }
}

#[tauri::command]
pub async fn mihomo_unfixed_proxy(group: String) -> Result<(), String> {
    crate::core::api::unfixed_proxy(&group).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mihomo_proxy_delay(proxy: String, url: Option<String>) -> Result<Value, String> {
    let config = crate::commands::config::get_app_config(None).await.unwrap_or(Value::Null);
    let default_url = config.get("delayTestUrl")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("https://cp.cloudflare.com");
    let test_url = url
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default_url.to_string());
    
    let timeout = config.get("delayTestTimeout")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(5000);

    crate::core::api::proxy_delay(&proxy, &test_url, timeout)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mihomo_group_delay(group: String, url: Option<String>) -> Result<Value, String> {
    let config = crate::commands::config::get_app_config(None).await.unwrap_or(Value::Null);
    let default_url = config.get("delayTestUrl")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("https://cp.cloudflare.com");
    let test_url = url
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default_url.to_string());
        
    let timeout = config.get("delayTestTimeout")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(5000);

    crate::core::api::group_delay(&group, &test_url, timeout)
        .await
        .map_err(|e| e.to_string())
}


#[tauri::command]
pub async fn mihomo_proxy_providers() -> Result<Value, String> {
    crate::core::api::get_proxy_providers().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mihomo_update_proxy_providers(name: String) -> Result<(), String> {
    crate::core::api::update_proxy_provider(&name)
        .await
        .map_err(|e| e.to_string())
}


#[tauri::command]
pub async fn mihomo_rule_providers() -> Result<Value, String> {
    crate::core::api::get_rule_providers().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mihomo_update_rule_providers(name: String) -> Result<(), String> {
    crate::core::api::update_rule_provider(&name)
        .await
        .map_err(|e| e.to_string())
}


#[tauri::command]
pub async fn mihomo_close_connection(id: String) -> Result<(), String> {
    crate::core::api::close_connection(&id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mihomo_close_all_connections(name: Option<String>) -> Result<(), String> {
    if let Some(host) = name {
        let cm = crate::core::api::connection_manager().map_err(|e| e.to_string())?;
        cm.close_by_host(&host).await.map(|_| ()).map_err(|e| e.to_string())
    } else {
        crate::core::api::close_all_connections().await.map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn restart_mihomo_connections() -> Result<(), String> {
    crate::core::api::restart_connections().await.map_err(|e| e.to_string())
}


#[tauri::command]
pub async fn mihomo_upgrade_geo() -> Result<(), String> {
    crate::core::api::upgrade_geo().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mihomo_upgrade_ui() -> Result<(), String> {
    crate::core::api::upgrade_ui().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn mihomo_upgrade(app: tauri::AppHandle, core: Option<String>) -> Result<(), String> {
    let selected = core.unwrap_or_else(|| "mihomo".to_string());
    crate::core::manager::install_core_for_core_type(&selected)
        .await
        .map_err(|e| e.to_string())?;
    crate::commands::core::restart_core(app).await
}
