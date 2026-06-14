use futures_util::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

use crate::backend::manager;

/// One update produced by the streaming loops.
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Raw payload of `GET /connections`.
    Connections(serde_json::Value),
    /// One parsed log line `{ "type": <level>, "payload": <msg> }`.
    Log(serde_json::Value),
}

/// Polls `/connections` once per second and forwards each snapshot.
pub async fn stream_connections(tx: UnboundedSender<StreamEvent>) {
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap_or_default();

    loop {
        if tx.is_closed() {
            return;
        }
        let url = manager::controller_url();
        if url.is_empty() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }
        let connections_url = format!("{url}/connections");
        match client.get(&connections_url).send().await {
            Ok(resp) => {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    if tx.send(StreamEvent::Connections(data)).is_err() {
                        return;
                    }
                }
            }
            Err(e) => log::debug!("[streaming] connections poll error: {e}"),
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Long-lived SSE-style reader of `/logs`. Reconnects on drop/error.
pub async fn stream_logs(tx: UnboundedSender<StreamEvent>) {
    loop {
        if tx.is_closed() {
            return;
        }
        let url = manager::controller_url();
        if url.is_empty() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        let logs_url = format!("{url}/logs?level=info");
        log::info!("[streaming] connecting to logs: {logs_url}");
        let client = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap_or_default();

        match client.get(&logs_url).send().await {
            Ok(resp) => {
                log::info!("[streaming] logs connected, status={}", resp.status());
                let mut stream = resp.bytes_stream();
                let mut buffer = String::new();
                while let Some(chunk) = stream.next().await {
                    if tx.is_closed() {
                        return;
                    }
                    if let Ok(bytes) = chunk {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer[..pos].trim().to_string();
                            buffer = buffer[pos + 1..].to_string();
                            if line.is_empty() {
                                continue;
                            }
                            if let Ok(entry) = serde_json::from_str::<serde_json::Value>(&line) {
                                if tx.send(StreamEvent::Log(entry)).is_err() {
                                    return;
                                }
                            }
                        }
                    }
                }
                log::info!("[streaming] logs stream ended");
            }
            Err(e) => log::warn!("[streaming] logs connection error: {e}"),
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
