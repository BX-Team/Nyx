use gpui::{App, AsyncApp};

use crate::app::runtime;
use crate::app::state::{self, AppState, CoreStatus};
use crate::backend;

/// Starts the mihomo core and live data flow. Call once after the main window
/// is open. Non-blocking: all work runs on the tokio runtime / gpui foreground.
pub fn spawn_backend_startup(cx: &mut App) {
    let state = AppState::global(cx);
    state.update(cx, |st, cx| st.set_core_status(CoreStatus::Starting, cx));

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<backend::streaming::StreamEvent>();

    // Drain the streaming channel into AppState on the gpui foreground.
    cx.spawn(async move |cx: &mut AsyncApp| {
        while let Some(ev) = rx.recv().await {
            cx.update(|cx| {
                AppState::global(cx).update(cx, |st, c| st.apply_stream_event(ev, c));
            });
        }
    })
    .detach();

    // Start the core, then streams + initial fetches.
    cx.spawn(async move |cx: &mut AsyncApp| {
        let outcome = runtime::spawn(backend::startup::start_core_flow()).await;
        let started = matches!(outcome, Ok(Ok(())));
        cx.update(|cx| {
            let state = AppState::global(cx);
            state.update(cx, |st, cx| match &outcome {
                Ok(Ok(())) => st.set_core_status(CoreStatus::Running, cx),
                Ok(Err(e)) => st.set_core_status(CoreStatus::Failed(e.clone().into()), cx),
                Err(_) => {
                    st.set_core_status(CoreStatus::Failed("startup task was cancelled".into()), cx)
                }
            });
        });

        if !started {
            log::error!("[bootstrap] core failed to start: {outcome:?}");
            return;
        }

        let tx_conn = tx.clone();
        runtime::detach(async move { backend::streaming::stream_connections(tx_conn).await });
        runtime::detach(async move { backend::streaming::stream_logs(tx).await });

        refresh_runtime_data(cx).await;
    })
    .detach();
}

/// Re-fetches groups, TUN state, mihomo version, and the current profile name.
pub async fn refresh_runtime_data(cx: &mut AsyncApp) {
    if let Ok(Ok(groups_val)) = runtime::spawn(backend::mihomo::groups()).await {
        cx.update(|cx| {
            let parsed = state::parse_groups(&groups_val);
            AppState::global(cx).update(cx, |st, c| st.set_groups(parsed, c));
        });
    }

    if let Ok(Ok(app_cfg)) = runtime::spawn(backend::config::get_app_config()).await {
        let autostart = app_cfg
            .get("autoStart")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        cx.update(|cx| {
            AppState::global(cx).update(cx, |st, c| st.set_app_config(app_cfg, c));
            crate::app::hotkeys::reload(cx);
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
