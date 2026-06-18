use std::sync::atomic::{AtomicBool, Ordering};

use gpui::{App, AsyncApp};

use crate::app::runtime;
use crate::app::state::{self, AppState, CoreStatus};
use crate::backend;

/// Streams are long-lived reconnect loops — wire them at most once per process.
static STREAMS_STARTED: AtomicBool = AtomicBool::new(false);

/// On launch: seed config, prefetch the binary, and — if setup is complete —
/// start the core (with TUN forced off, so opening the app never connects).
pub fn spawn_backend_startup(cx: &mut App) {
    cx.spawn(async move |cx: &mut AsyncApp| {
        let _ = runtime::spawn(async { backend::startup::ensure_default_app_config() }).await;
        prefetch_core_binary();

        refresh_profiles(cx).await;

        if can_autostart_core().await {
            if crate::app::autostart::launched_at_boot()
                && backend::config::app_config_bool("lastConnected")
            {
                start_core_connected(cx).await;
            } else {
                start_core_disconnected(cx).await;
            }
            return;
        }

        cx.update(|cx| {
            AppState::global(cx).update(cx, |st, cx| st.set_core_status(CoreStatus::Stopped, cx));
        });
        refresh_runtime_data(cx).await;
    })
    .detach();
}

/// Core may be started unattended only once a profile exists and the runtime is
/// available (Windows service installed / core binary present).
async fn can_autostart_core() -> bool {
    if !has_any_profile().await {
        return false;
    }
    matches!(
        runtime::spawn(backend::service::service_status()).await,
        Ok(Ok(s)) if s != "not-installed"
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

pub async fn start_core_disconnected(cx: &mut AsyncApp) -> bool {
    let _ = runtime::spawn(backend::config::patch_controled_mihomo_config(
        serde_json::json!({ "tun": { "enable": false } }),
    ))
    .await;
    start_core_and_streams(cx).await
}

pub async fn start_core_connected(cx: &mut AsyncApp) -> bool {
    let _ = runtime::spawn(backend::config::patch_controled_mihomo_config(
        serde_json::json!({ "tun": { "enable": true }, "dns": { "enable": true } }),
    ))
    .await;
    start_core_and_streams(cx).await
}

pub async fn start_core_and_streams(cx: &mut AsyncApp) -> bool {
    cx.update(|cx| {
        AppState::global(cx).update(cx, |st, cx| st.set_core_status(CoreStatus::Starting, cx));
    });

    let outcome = runtime::spawn(backend::startup::start_core_flow()).await;
    let started = matches!(outcome, Ok(Ok(())));
    cx.update(|cx| {
        AppState::global(cx).update(cx, |st, cx| match &outcome {
            Ok(Ok(())) => st.set_core_status(CoreStatus::Running, cx),
            Ok(Err(e)) => st.set_core_status(CoreStatus::Failed(e.clone().into()), cx),
            Err(_) => {
                st.set_core_status(CoreStatus::Failed("startup task was cancelled".into()), cx)
            }
        });
    });

    if !started {
        log::error!("[bootstrap] core failed to start: {outcome:?}");
        return false;
    }

    wait_for_core_ready().await;

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

/// Polls the core's HTTP controller until it answers — mihomo needs a moment to
/// bind after spawn, else the first groups/version fetch comes back empty.
async fn wait_for_core_ready() {
    let _ = runtime::spawn(async {
        for _ in 0..40 {
            if backend::api::get_version().await.is_ok() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }
    })
    .await;
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

/// Re-fetches groups, TUN state, mihomo version, and the current profile name.
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

    if let Ok(Ok(version)) = runtime::spawn(backend::api::get_version()).await {
        cx.update(|cx| {
            AppState::global(cx).update(cx, |st, c| {
                st.mihomo_version = Some(version.into());
                c.notify();
            });
        });
    }

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

/// Returns the full JSON of the currently selected profile item.
fn current_profile_item(pcfg: &serde_json::Value) -> Option<serde_json::Value> {
    let current = pcfg.get("current").and_then(|v| v.as_str())?;
    pcfg.get("items")
        .and_then(|v| v.as_array())?
        .iter()
        .find(|it| it.get("id").and_then(|v| v.as_str()) == Some(current))
        .cloned()
}

/// Resolves the display name of the currently selected profile.
fn current_profile_name(pcfg: &serde_json::Value) -> Option<gpui::SharedString> {
    let current = pcfg.get("current").and_then(|v| v.as_str())?;
    let items = pcfg.get("items").and_then(|v| v.as_array())?;
    items
        .iter()
        .find(|it| it.get("id").and_then(|v| v.as_str()) == Some(current))
        .and_then(|it| it.get("name").and_then(|v| v.as_str()))
        .map(|s| s.to_string().into())
}
