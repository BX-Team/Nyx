use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;

static STREAMING_ACTIVE: Lazy<AtomicBool> = Lazy::new(|| AtomicBool::new(false));

pub fn start_streaming(app: &AppHandle) {
    if STREAMING_ACTIVE.swap(true, Ordering::SeqCst) {
        return; 
    }
    log::info!("[streaming] starting connection and log streams");

    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        stream_connections(handle).await;
    });

    let handle2 = app.clone();
    tauri::async_runtime::spawn(async move {
        stream_logs(handle2).await;
    });
}

pub fn stop_streaming() {
    log::info!("[streaming] stopping streams");
    STREAMING_ACTIVE.store(false, Ordering::SeqCst);
}

async fn stream_connections(app: AppHandle) {
    use tauri::Emitter;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    while STREAMING_ACTIVE.load(Ordering::SeqCst) {
        let url = super::manager::controller_url();
        if url.is_empty() {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            continue;
        }
        let connections_url = format!("{url}/connections");
        match client.get(&connections_url).send().await {
            Ok(resp) => {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    let _ = app.emit("mihomo-connections", &data);
                }
            }
            Err(e) => {
                log::debug!("[streaming] connections poll error: {e}");
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    log::info!("[streaming] connections stream stopped");
}

async fn stream_logs(app: AppHandle) {
    use tauri::Emitter;

    while STREAMING_ACTIVE.load(Ordering::SeqCst) {
        let url = super::manager::controller_url();
        if url.is_empty() {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            continue;
        }

        let logs_url = format!("{url}/logs?level=info");
        log::info!("[streaming] connecting to logs: {logs_url}");

        let client = reqwest::Client::builder()
            .build()
            .unwrap_or_default();

        match client.get(&logs_url).send().await {
            Ok(resp) => {
                log::info!("[streaming] logs connected, status={}", resp.status());
                use futures_util::StreamExt;
                let mut stream = resp.bytes_stream();
                let mut buffer = String::new();
                while let Some(chunk) = stream.next().await {
                    if !STREAMING_ACTIVE.load(Ordering::SeqCst) {
                        break;
                    }
                    if let Ok(bytes) = chunk {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer[..pos].trim().to_string();
                            buffer = buffer[pos + 1..].to_string();
                            if !line.is_empty() {
                                if let Ok(log_entry) = serde_json::from_str::<serde_json::Value>(&line) {
                                    let _ = app.emit("mihomo-logs", &log_entry);
                                }
                            }
                        }
                    }
                }
                log::info!("[streaming] logs stream ended");
            }
            Err(e) => {
                log::warn!("[streaming] logs connection error: {e}");
            }
        }
        if STREAMING_ACTIVE.load(Ordering::SeqCst) {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
    log::info!("[streaming] logs stream stopped");
}
