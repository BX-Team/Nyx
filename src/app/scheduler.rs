use std::time::Duration;

use gpui::{App, AsyncApp};
use rust_i18n::t;

use crate::app::runtime;
use crate::app::state::AppState;
use crate::backend;

const CHECK_INTERVAL_SECS: u64 = 300;
const INITIAL_DELAY_SECS: u64 = 15;
const EXPIRY_WARN_SECS: i64 = 3 * 86_400;

/// Starts the background scheduler: a one-time quota/expiry check shortly after
/// startup, then a recurring sweep that re-downloads any remote profile whose
/// auto-update interval has elapsed.
pub fn init(cx: &mut App) {
    cx.spawn(async move |cx: &mut AsyncApp| {
        cx.background_executor()
            .timer(Duration::from_secs(INITIAL_DELAY_SECS))
            .await;
        quota_check(cx);
        loop {
            run_due(cx).await;
            cx.background_executor()
                .timer(Duration::from_secs(CHECK_INTERVAL_SECS))
                .await;
        }
    })
    .detach();
}

async fn run_due(cx: &mut AsyncApp) {
    let updated = runtime::spawn(backend::config::run_due_auto_updates())
        .await
        .unwrap_or_default();
    if updated.is_empty() {
        return;
    }
    log::info!("[scheduler] auto-updated profiles: {}", updated.join(", "));
    crate::app::bootstrap::refresh_runtime_data(cx).await;
    cx.update(|cx| {
        let msg = format!(
            "{}: {}",
            t!("pages.profiles.autoUpdated"),
            updated.join(", ")
        );
        crate::app::actions::notify(gpui_component::notification::Notification::info(msg), cx);
    });
}

fn quota_check(cx: &mut AsyncApp) {
    cx.update(|cx| {
        let profiles = AppState::global(cx).read(cx).profiles.clone();
        let now = chrono::Utc::now().timestamp();
        for p in &profiles {
            if p.total > 0 && (p.used as u128) * 100 >= (p.total as u128) * 90 {
                crate::app::actions::notify(
                    gpui_component::notification::Notification::warning(
                        t!("pages.profiles.quotaWarn", name => p.name.clone()).to_string(),
                    ),
                    cx,
                );
            }
            if p.expire > now && p.expire - now < EXPIRY_WARN_SECS {
                crate::app::actions::notify(
                    gpui_component::notification::Notification::warning(
                        t!("pages.profiles.expiryWarn", name => p.name.clone()).to_string(),
                    ),
                    cx,
                );
            }
        }
    });
}
