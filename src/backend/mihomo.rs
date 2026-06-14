use serde_json::Value;

use crate::backend::{api, config, dirs, manager};

/// Returns proxy groups as JSON objects, each with its `all` members resolved to
/// full proxy objects.
pub async fn groups() -> anyhow::Result<Value> {
    let proxies = api::get_raw_proxies_map().await?;

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
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });

    Ok(Value::Array(groups))
}

pub async fn change_proxy(group: &str, proxy: &str) -> anyhow::Result<()> {
    let encoded_group = group.replace(' ', "%20");
    let url = format!("{}/proxies/{}", manager::controller_url(), encoded_group);
    reqwest::Client::new()
        .put(&url)
        .json(&serde_json::json!({ "name": proxy }))
        .send()
        .await?
        .error_for_status()?;
    save_selection(group, proxy);
    Ok(())
}

fn save_selection(group: &str, proxy: &str) {
    let path = dirs::selections_path();
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
    let path = dirs::selections_path();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(map) = serde_yaml::from_str::<serde_yaml::Mapping>(&text) else {
        return;
    };
    let base_url = manager::controller_url();
    if base_url.is_empty() {
        return;
    }

    // Selections are stored globally across profiles, so validate each against
    // the live proxy set first — a stale group/proxy would 404 if restored blindly.
    let proxies = match api::get_raw_proxies_map().await {
        Ok(p) => p,
        Err(e) => {
            log::warn!("[restore_proxy_selections] could not fetch proxies: {e}");
            return;
        }
    };

    let client = reqwest::Client::new();
    for (k, v) in map.iter() {
        let (Some(group), Some(proxy)) = (k.as_str(), v.as_str()) else {
            continue;
        };

        let Some(group_info) = proxies.get(group) else {
            log::debug!("[restore_proxy_selections] skip {group}: group not in current profile");
            continue;
        };
        if group_info["type"].as_str() != Some("Selector") {
            continue;
        }
        let is_member = group_info["all"]
            .as_array()
            .map(|all| all.iter().any(|n| n.as_str() == Some(proxy)))
            .unwrap_or(false);
        if !is_member {
            log::debug!("[restore_proxy_selections] skip {group}: '{proxy}' no longer a member");
            continue;
        }

        let encoded_group = group.replace(' ', "%20");
        let url = format!("{}/proxies/{}", base_url, encoded_group);
        match client
            .put(&url)
            .json(&serde_json::json!({ "name": proxy }))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => {
                log::info!("[restore_proxy_selections] {group} -> {proxy}");
            }
            Ok(r) => {
                log::warn!(
                    "[restore_proxy_selections] {group} -> {proxy} returned {}",
                    r.status()
                );
            }
            Err(e) => {
                log::warn!("[restore_proxy_selections] {group} -> {proxy} failed: {e}");
            }
        }
    }
}

async fn delay_test_params(url: Option<String>) -> (String, u32) {
    let cfg = config::get_app_config().await.unwrap_or(Value::Null);
    let default_url = cfg
        .get("delayTestUrl")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("https://cp.cloudflare.com");
    let test_url = url
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default_url.to_string());
    let timeout = cfg
        .get("delayTestTimeout")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(5000);
    (test_url, timeout)
}

pub async fn proxy_delay(proxy: &str, url: Option<String>) -> anyhow::Result<Value> {
    let (test_url, timeout) = delay_test_params(url).await;
    api::proxy_delay(proxy, &test_url, timeout).await
}

pub async fn group_delay(group: &str, url: Option<String>) -> anyhow::Result<Value> {
    let (test_url, timeout) = delay_test_params(url).await;
    api::group_delay(group, &test_url, timeout).await
}
