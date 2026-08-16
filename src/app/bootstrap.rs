use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use gpui::{App, AsyncApp};

use crate::app::runtime;
use crate::app::state::{self, AppState, CoreStatus};
use crate::backend;
use crate::backend::core::ServiceStatus;

/// Streams are long-lived reconnect loops — wire them at most once per process.
static STREAMS_STARTED: AtomicBool = AtomicBool::new(false);

const WATCH_INTERVAL: Duration = Duration::from_secs(5);
const RETRY_MIN: Duration = Duration::from_secs(5);
const RETRY_MAX: Duration = Duration::from_secs(300);

/// On launch: seed config, prefetch the binary, then start the core with the
/// connection state from the last session.
pub fn spawn_backend_startup(cx: &mut App) {
    cx.spawn(async move |cx: &mut AsyncApp| {
        let _ = runtime::spawn(async {
            backend::startup::ensure_default_app_config();
            backend::startup::normalize_connection_mode().await;
        })
        .await;
        prefetch_core_binary();
        refresh_profiles(cx).await;

        if !can_autostart_core().await {
            cx.update(|cx| {
                AppState::global(cx)
                    .update(cx, |st, cx| st.set_core_status(CoreStatus::Stopped, cx));
            });
            refresh_runtime_data(cx).await;
            return;
        }

        start_core_and_streams(cx, restore_tun()).await;
        watch_core(cx).await;
    })
    .detach();
}

/// The single restart owner: revives a dead core and retries a failed start
/// with a growing delay.
async fn watch_core(cx: &mut AsyncApp) {
    let mut backoff = RETRY_MIN;
    let mut waited = Duration::ZERO;

    loop {
        cx.background_executor().timer(WATCH_INTERVAL).await;

        let mut status = CoreStatus::Stopped;
        cx.update(|cx| status = AppState::global(cx).read(cx).core_status.clone());

        match status {
            CoreStatus::Running => {
                waited = Duration::ZERO;
                backoff = RETRY_MIN;
                if runtime::spawn(backend::core::is_alive()).await == Ok(false) {
                    log::warn!("[watchdog] the core is gone, restarting it");
                    restart_core(cx).await;
                }
            }
            CoreStatus::Failed { .. } => {
                waited += WATCH_INTERVAL;
                if waited < backoff {
                    continue;
                }
                waited = Duration::ZERO;
                backoff = (backoff * 2).min(RETRY_MAX);
                log::info!("[watchdog] retrying core start");
                restart_core(cx).await;
            }
            _ => {}
        }
    }
}

async fn restart_core(cx: &mut AsyncApp) {
    start_core_and_streams(cx, restore_tun()).await;
}

pub fn restore_tun() -> bool {
    backend::config::app_config_bool("lastConnected")
        && backend::config::app_config_str("connectionMode", "tun") == "tun"
}

/// Unattended start needs a profile plus a runtime — the service, or direct mode.
async fn can_autostart_core() -> bool {
    if !has_any_profile().await {
        return false;
    }
    if backend::config::app_config_str("corePermissionMode", "service") == "direct" {
        return true;
    }
    !matches!(
        runtime::spawn(backend::core::service_status()).await,
        Ok(ServiceStatus::NotInstalled)
    )
}

async fn has_any_profile() -> bool {
    matches!(
        runtime::spawn(backend::config::get_profile_config()).await,
        Ok(Ok(cfg)) if cfg
            .get("items")
            .and_then(|v| v.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false)
    )
}

pub async fn start_core_and_streams(cx: &mut AsyncApp, connected: bool) -> bool {
    let tun = serde_json::json!({ "tun": { "enable": connected } });
    let patch = if connected {
        serde_json::json!({ "tun": { "enable": true }, "dns": { "enable": true } })
    } else {
        tun
    };
    let _ = runtime::spawn(backend::config::patch_controled_mihomo_config(patch)).await;

    cx.update(|cx| {
        AppState::global(cx).update(cx, |st, cx| st.set_core_status(CoreStatus::Starting, cx));
    });

    let outcome = runtime::spawn(backend::startup::start_core_flow()).await;
    let started = matches!(outcome, Ok(Ok(())));
    cx.update(|cx| {
        AppState::global(cx).update(cx, |st, cx| match &outcome {
            Ok(Ok(())) => st.set_core_status(CoreStatus::Running, cx),
            Ok(Err(e)) => st.set_core_status(e.clone().into(), cx),
            Err(_) => st.set_core_status(
                backend::core::CoreError::new(
                    backend::core::FailureKind::Other,
                    "the startup task was cancelled",
                )
                .into(),
                cx,
            ),
        });
    });

    if !started {
        log::error!("[bootstrap] core failed to start");
        return false;
    }

    if !STREAMS_STARTED.swap(true, Ordering::SeqCst) {
        let (tx, mut rx) =
            tokio::sync::mpsc::unbounded_channel::<backend::streaming::StreamEvent>();
        cx.update(|cx| {
            cx.spawn(async move |cx: &mut AsyncApp| {
                while let Some(ev) = rx.recv().await {
                    cx.update(|cx| {
                        AppState::global(cx).update(cx, |st, c| st.apply_stream_event(ev, c));
                    });
                }
            })
            .detach();
        });
        let tx_conn = tx.clone();
        runtime::detach(async move { backend::streaming::stream_connections(tx_conn).await });
        runtime::detach(async move { backend::streaming::stream_logs(tx).await });
    }

    refresh_runtime_data(cx).await;

    // Re-apply saved system-proxy now the core is up; also clears any left by a crash.
    if let Ok(Ok(cfg)) = runtime::spawn(backend::config::get_app_config()).await {
        let enable = cfg
            .get("sysProxy")
            .and_then(|v| v.get("enable"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let affect = cfg
            .get("affectVPNConnections")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let _ = runtime::spawn(backend::sysproxy::apply(enable, affect)).await;
    }
    true
}

fn prefetch_core_binary() {
    let core = backend::config::app_config_str("core", "mihomo");
    if core == "system" {
        return;
    }
    runtime::detach(async move {
        if backend::manager::core_installed().await {
            return;
        }
        log::info!("[bootstrap] prefetching mihomo core binary ({core})");
        if let Err(e) = backend::manager::install_core_for_core_type(&core).await {
            log::warn!("[bootstrap] core prefetch failed: {e}");
        }
    });
}

pub async fn refresh_runtime_data(cx: &mut AsyncApp) {
    if let Ok(Ok(groups_val)) = runtime::spawn(backend::mihomo::groups()).await {
        cx.update(|cx| {
            let parsed = state::parse_groups(&groups_val);
            AppState::global(cx).update(cx, |st, c| st.set_groups(parsed, c));
            crate::app::tray::rebuild(cx);
        });
    }

    if let Ok(Ok(app_cfg)) = runtime::spawn(backend::config::get_app_config()).await {
        let flag = |k: &str| app_cfg.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
        let autostart = flag("autoStart");
        let on_top = flag("alwaysOnTop");
        let tray_enabled = !flag("disableTray");
        cx.update(|cx| {
            AppState::global(cx).update(cx, |st, c| st.set_app_config(app_cfg.clone(), c));
            crate::app::hotkeys::reload(cx);
            crate::app::tray::set_enabled(cx, tray_enabled);
            // Win32 SetWindowPos must run outside the live gpui borrow.
            cx.spawn(async move |_cx| crate::app::window::set_always_on_top(on_top))
                .detach();
        });
        crate::app::autostart::sync(autostart);
    }

    if let Ok(Ok(rules_val)) = runtime::spawn(backend::api::get_rules()).await {
        cx.update(|cx| {
            let parsed = state::parse_rules(&rules_val);
            AppState::global(cx).update(cx, |st, c| st.set_rules(parsed, c));
        });
    }

    if let Ok(Ok(cfg)) = runtime::spawn(backend::config::get_controled_mihomo_config()).await {
        let tun = cfg
            .get("tun")
            .and_then(|t| t.get("enable"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let mode = cfg
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("rule")
            .to_string();
        cx.update(|cx| {
            AppState::global(cx).update(cx, |st, c| {
                let tun = tun && st.core_status.is_running();
                st.set_tun_enabled(tun, c);
                st.set_mode(mode, c);
                st.set_controled_config(cfg.clone(), c);
            });
        });
    }

    let version = runtime::spawn(backend::api::get_version()).await;
    cx.update(|cx| {
        AppState::global(cx).update(cx, |st, c| {
            st.mihomo_version = match &version {
                Ok(Ok(v)) => Some(v.clone().into()),
                _ => None,
            };
            c.notify();
        });
    });

    refresh_profiles(cx).await;
}

pub async fn refresh_profiles(cx: &mut AsyncApp) {
    if let Ok(Ok(pcfg)) = runtime::spawn(backend::config::get_profile_config()).await {
        let name = current_profile_name(&pcfg);
        let profiles = state::parse_profiles(&pcfg);
        let item = current_profile_item(&pcfg);
        cx.update(|cx| {
            AppState::global(cx).update(cx, |st, c| {
                st.set_current_profile_name(name, c);
                st.set_profiles(profiles, c);
                st.set_current_profile_item(item, c);
            });
        });
    }
}

fn current_profile_item(pcfg: &serde_json::Value) -> Option<serde_json::Value> {
    let current = pcfg.get("current").and_then(|v| v.as_str())?;
    pcfg.get("items")
        .and_then(|v| v.as_array())?
        .iter()
        .find(|it| it.get("id").and_then(|v| v.as_str()) == Some(current))
        .cloned()
}

fn current_profile_name(pcfg: &serde_json::Value) -> Option<gpui::SharedString> {
    let current = pcfg.get("current").and_then(|v| v.as_str())?;
    let items = pcfg.get("items").and_then(|v| v.as_array())?;
    items
        .iter()
        .find(|it| it.get("id").and_then(|v| v.as_str()) == Some(current))
        .and_then(|it| it.get("name").and_then(|v| v.as_str()))
        .map(|s| s.to_string().into())
}
