mod process;
mod spec;

use std::time::Duration;

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use tokio::sync::Mutex as AsyncMutex;

pub use nyx_service::Status as ServiceStatus;

use crate::backend::{api, config};

/// Generous — a large rule-provider set takes a while to load.
const READY_TIMEOUT: Duration = Duration::from_secs(20);
const READY_POLL: Duration = Duration::from_millis(200);
/// Liveness is far more expensive than an HTTP poll, so check it less often.
const LIVENESS_EVERY: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    CoreMissing,
    ConfigInvalid,
    ServiceUnavailable,
    Timeout,
    Other,
}

#[derive(Debug, Clone)]
pub struct CoreError {
    pub kind: FailureKind,
    pub detail: String,
}

impl CoreError {
    pub fn new(kind: FailureKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.detail)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Service,
    Direct,
}

static CONTROLLER_URL: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::new()));
/// Which backend started the core that is running now, so `stop` targets it.
static ACTIVE: Lazy<Mutex<Option<Backend>>> = Lazy::new(|| Mutex::new(None));
/// Serialises start/stop/restart; the UI, the tray and the watchdog all call in.
static TRANSITION: Lazy<AsyncMutex<()>> = Lazy::new(|| AsyncMutex::new(()));

pub fn controller_url() -> String {
    CONTROLLER_URL.lock().clone()
}

pub fn set_controller_url(url: String) {
    *CONTROLLER_URL.lock() = url;
}

pub async fn service_status() -> ServiceStatus {
    nyx_service::status().await.unwrap_or_else(|reason| {
        log::warn!("[core] service status query failed: {reason}");
        ServiceStatus::Stale { reason }
    })
}

pub async fn install_service() -> Result<(), CoreError> {
    nyx_service::install()
        .await
        .map_err(|e| CoreError::new(FailureKind::ServiceUnavailable, e))
}

pub async fn uninstall_service() -> Result<(), CoreError> {
    let _ = stop().await;
    nyx_service::uninstall()
        .await
        .map_err(|e| CoreError::new(FailureKind::ServiceUnavailable, e))
}

/// Brings the core up and does not return `Ok` until its controller answers.
pub async fn start() -> Result<(), CoreError> {
    let _lock = TRANSITION.lock().await;
    start_locked().await
}

pub async fn stop() -> Result<(), CoreError> {
    let _lock = TRANSITION.lock().await;
    stop_locked().await;
    Ok(())
}

pub async fn restart() -> Result<(), CoreError> {
    let _lock = TRANSITION.lock().await;
    stop_locked().await;
    start_locked().await
}

async fn start_locked() -> Result<(), CoreError> {
    let backend = choose_backend().await?;
    let spec = spec::build().await?;
    log::info!(
        "[core] starting via {backend:?}: binary={:?} config={:?} controller={}",
        spec.binary,
        spec.config,
        spec.controller
    );

    match backend {
        Backend::Service => {
            let request = nyx_service::CoreSpec {
                binary: spec.binary.clone(),
                work_dir: spec.work_dir.clone(),
                config: spec.config.clone(),
                max_log_days: spec.max_log_days,
            };
            nyx_service::start_core(&request)
                .await
                .map_err(|e| CoreError::new(FailureKind::ServiceUnavailable, e))?;
        }
        Backend::Direct => {
            process::spawn(&spec).await?;
        }
    }
    *ACTIVE.lock() = Some(backend);

    set_controller_url(spec.controller.clone());
    api::init_client(&spec.controller, spec.secret.clone())
        .map_err(|e| CoreError::new(FailureKind::Other, e.to_string()))?;

    wait_ready(backend, &spec.controller).await?;
    log::info!("[core] ready at {}", spec.controller);
    Ok(())
}

async fn stop_locked() {
    let active = ACTIVE.lock().take();
    match active {
        Some(Backend::Service) => {
            if let Err(e) = nyx_service::stop_core().await {
                log::warn!("[core] service stop failed: {e}");
            }
        }
        Some(Backend::Direct) => process::stop().await,
        // Unknown owner (fresh launch): clear both, so a previous run cannot keep
        // the proxy alive behind us.
        None => {
            let _ = nyx_service::stop_core().await;
            process::stop().await;
        }
    }
}

async fn choose_backend() -> Result<Backend, CoreError> {
    if config::app_config_str("corePermissionMode", "service") == "direct" {
        return Ok(Backend::Direct);
    }
    match service_status().await {
        ServiceStatus::NotInstalled => Err(CoreError::new(
            FailureKind::ServiceUnavailable,
            "the Nyx service is not installed",
        )),
        _ => Ok(Backend::Service),
    }
}

/// Polls the controller until it answers, giving up early if the core died.
async fn wait_ready(backend: Backend, url: &str) -> Result<(), CoreError> {
    let deadline = tokio::time::Instant::now() + READY_TIMEOUT;
    let mut tick = 0u32;
    loop {
        if api_responds(url).await {
            return Ok(());
        }
        tick += 1;
        if tick.is_multiple_of(LIVENESS_EVERY) && !backend_alive(backend).await {
            return Err(CoreError::new(
                FailureKind::ConfigInvalid,
                "the core exited right after starting — check the core log",
            ));
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(CoreError::new(
                FailureKind::Timeout,
                format!("the core did not answer on {url} within 20s"),
            ));
        }
        tokio::time::sleep(READY_POLL).await;
    }
}

async fn backend_alive(backend: Backend) -> bool {
    match backend {
        Backend::Service => matches!(nyx_service::ping().await, Ok(Some(_))),
        Backend::Direct => process::alive(),
    }
}

pub async fn is_alive() -> bool {
    let active = *ACTIVE.lock();
    match active {
        Some(backend) => backend_alive(backend).await,
        None => false,
    }
}

pub async fn is_ready() -> bool {
    let url = controller_url();
    !url.is_empty() && api_responds(&url).await
}

async fn api_responds(url: &str) -> bool {
    let Ok(client) = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()
    else {
        return false;
    };
    client
        .get(format!("{url}/version"))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}
