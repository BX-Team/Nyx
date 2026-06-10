use serde::{Deserialize, Serialize};

const REPO_OWNER: &str = "BX-Team";
const REPO_NAME: &str = "Nyx";

/// A newer release available for install.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub changelog: String,
}

/// Returns the newest release if it is newer than the running build, else `None`.
pub async fn check() -> Result<Option<UpdateInfo>, String> {
    tokio::task::spawn_blocking(|| {
        let releases = self_update::backends::github::ReleaseList::configure()
            .repo_owner(REPO_OWNER)
            .repo_name(REPO_NAME)
            .build()
            .map_err(|e| e.to_string())?
            .fetch()
            .map_err(|e| e.to_string())?;

        let Some(latest) = releases.into_iter().next() else {
            return Ok(None);
        };
        let current = self_update::cargo_crate_version!();
        let newer = self_update::version::bump_is_greater(current, &latest.version)
            .map_err(|e| e.to_string())?;
        if newer {
            Ok(Some(UpdateInfo {
                version: latest.version,
                changelog: latest.body.unwrap_or_default(),
            }))
        } else {
            Ok(None)
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Downloads the matching release asset and self-replaces the running binary.
/// On Windows the service is stopped first so it doesn't hold a file lock.
pub async fn download_and_install() -> Result<(), String> {
    #[cfg(windows)]
    {
        let _ = crate::backend::service::stop_service_for_update().await;
    }

    tokio::task::spawn_blocking(|| {
        self_update::backends::github::Update::configure()
            .repo_owner(REPO_OWNER)
            .repo_name(REPO_NAME)
            .bin_name("nyx")
            .current_version(self_update::cargo_crate_version!())
            .no_confirm(true)
            .show_download_progress(false)
            .build()
            .map_err(|e| e.to_string())?
            .update()
            .map_err(|e| e.to_string())?;
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())?
}
