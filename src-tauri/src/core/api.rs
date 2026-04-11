use anyhow::Result;
use mihomo_rs::{ConnectionManager, MihomoClient, ProxyManager};
use once_cell::sync::Lazy;
use parking_lot::Mutex;

static CLIENT: Lazy<Mutex<Option<MihomoClient>>> = Lazy::new(|| Mutex::new(None));

pub fn init_client(url: &str, secret: Option<String>) -> Result<()> {
    let client = MihomoClient::new(url, secret).map_err(|e| anyhow::anyhow!("{e}"))?;
    *CLIENT.lock() = Some(client);
    Ok(())
}

pub fn get_client() -> Result<MihomoClient> {
    CLIENT
        .lock()
        .clone()
        .ok_or_else(|| anyhow::anyhow!("mihomo client not initialised — is the core running?"))
}

pub fn proxy_manager() -> Result<ProxyManager> {
    Ok(ProxyManager::new(get_client()?))
}

pub fn connection_manager() -> Result<ConnectionManager> {
    Ok(ConnectionManager::new(get_client()?))
}


pub async fn get_version() -> Result<String> {
    let v = get_client()?.get_version().await.map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(v.version)
}

pub async fn get_proxies() -> Result<serde_json::Value> {
    let proxies = get_client()?.get_proxies().await.map_err(|e| anyhow::anyhow!("{e}"))?;
    serde_json::to_value(proxies).map_err(|e| anyhow::anyhow!("{e}"))
}

pub async fn proxy_delay(proxy: &str, test_url: &str, timeout: u32) -> Result<serde_json::Value> {
    let encoded = proxy.replace(' ', "%20");
    let encoded_url: String = url::form_urlencoded::byte_serialize(test_url.as_bytes()).collect();
    let url = format!(
        "{}/proxies/{}/delay?url={}&timeout={}",
        base_url(),
        encoded,
        encoded_url,
        timeout
    );
    let resp: serde_json::Value = http()
        .get(&url)
        .send()
        .await?
        .json()
        .await?;
    Ok(resp)
}

pub async fn close_connection(id: &str) -> Result<()> {
    get_client()?.close_connection(id).await.map_err(|e| anyhow::anyhow!("{e}"))
}

pub async fn close_all_connections() -> Result<()> {
    get_client()?.close_all_connections().await.map_err(|e| anyhow::anyhow!("{e}"))
}

pub async fn get_connections() -> Result<serde_json::Value> {
    let conns = get_client()?.get_connections().await.map_err(|e| anyhow::anyhow!("{e}"))?;
    serde_json::to_value(conns).map_err(|e| anyhow::anyhow!("{e}"))
}

pub async fn reload_config(path: Option<&str>) -> Result<()> {
    get_client()?.reload_config(path).await.map_err(|e| anyhow::anyhow!("{e}"))
}


fn base_url() -> String {
    crate::core::manager::controller_url()
}

fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap_or_default()
}

pub async fn get_config() -> Result<serde_json::Value> {
    let url = format!("{}/configs", base_url());
    Ok(http().get(&url).send().await?.json().await?)
}

pub async fn patch_config(patch: serde_json::Value) -> Result<()> {
    let url = format!("{}/configs", base_url());
    http().patch(&url).json(&patch).send().await?;
    Ok(())
}

pub async fn get_rules() -> Result<serde_json::Value> {
    let url = format!("{}/rules", base_url());
    Ok(http().get(&url).send().await?.json().await?)
}

pub async fn get_raw_proxies_map() -> Result<std::collections::HashMap<String, serde_json::Value>> {
    let url = format!("{}/proxies", base_url());
    let resp: serde_json::Value = http().get(&url).send().await?.json().await?;
    let map = resp["proxies"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("missing 'proxies' field in /proxies response"))?
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    Ok(map)
}

pub async fn get_proxy_providers() -> Result<serde_json::Value> {
    let url = format!("{}/providers/proxies", base_url());
    Ok(http().get(&url).send().await?.json().await?)
}

pub async fn update_proxy_provider(name: &str) -> Result<()> {
    let url = format!("{}/providers/proxies/{}", base_url(), name);
    http().put(&url).send().await?;
    Ok(())
}

pub async fn get_rule_providers() -> Result<serde_json::Value> {
    let url = format!("{}/providers/rules", base_url());
    Ok(http().get(&url).send().await?.json().await?)
}

pub async fn update_rule_provider(name: &str) -> Result<()> {
    let url = format!("{}/providers/rules/{}", base_url(), name);
    http().put(&url).send().await?;
    Ok(())
}

pub async fn group_delay(group: &str, test_url: &str, timeout: u32) -> Result<serde_json::Value> {
    let encoded = group.replace(' ', "%20");
    let encoded_url: String = url::form_urlencoded::byte_serialize(test_url.as_bytes()).collect();
    let url = format!(
        "{}/group/{}/delay?url={}&timeout={}",
        base_url(),
        encoded,
        encoded_url,
        timeout
    );
    Ok(http().get(&url).send().await?.json().await?)
}

pub async fn upgrade_geo() -> Result<()> {
    let url = format!("{}/configs/geo", base_url());
    http().patch(&url).send().await?;
    Ok(())
}

pub async fn upgrade_ui() -> Result<()> {
    let url = format!("{}/upgrade/ui", base_url());
    http().post(&url).send().await?;
    Ok(())
}

pub async fn unfixed_proxy(group: &str) -> Result<()> {
    let url = format!("{}/proxies/{}", base_url(), group);
    http()
        .put(&url)
        .json(&serde_json::json!({ "name": "DIRECT" }))
        .send()
        .await?;
    Ok(())
}

pub async fn restart_connections() -> Result<()> {
    close_all_connections().await
}
