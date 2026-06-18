use serde::{Deserialize, Serialize};

const REPO_OWNER: &str = "BX-Team";
const REPO_NAME: &str = "Nyx";

#[cfg(windows)]
const WINDOWS_ASSET: &str = "Nyx-x86_64-windows.zip";

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

/// Downloads and installs the newest release.
///
/// Returns `true` when the relaunch is handled externally and the caller should
/// just leave it to the helper, or `false` when the binary was replaced in place
/// and the caller should relaunch itself.
///
/// On Windows the app lives in `Program Files` and the running `nyx.exe` (plus
/// the background `--nyx-service` process) lock the file, so a non-elevated
/// in-place replace fails with "access denied". Instead we download + unpack the
/// new binary to a temp dir and hand off to a short elevated script that kills
/// the running processes, overwrites the installed exe, and relaunches it.
pub async fn download_and_install() -> Result<bool, String> {
    #[cfg(windows)]
    {
        let _ = crate::backend::service::stop_service_for_update().await;
        windows_update().await
    }

    #[cfg(not(windows))]
    {
        tokio::task::spawn_blocking(|| {
            self_update::backends::github::Update::configure()
                .repo_owner(REPO_OWNER)
                .repo_name(REPO_NAME)
                .target("x86_64-linux")
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
        .map_err(|e| e.to_string())??;
        Ok(false)
    }
}

#[cfg(windows)]
async fn windows_update() -> Result<bool, String> {
    let url = tokio::task::spawn_blocking(windows_asset_url)
        .await
        .map_err(|e| e.to_string())??;

    let client = reqwest::Client::builder()
        .user_agent(concat!("Nyx/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let bytes = client
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    tokio::task::spawn_blocking(move || finalize_windows_update(&bytes))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(windows)]
fn windows_asset_url() -> Result<String, String> {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()
        .map_err(|e| e.to_string())?
        .fetch()
        .map_err(|e| e.to_string())?;
    let latest = releases.into_iter().next().ok_or("no releases found")?;
    latest
        .assets
        .into_iter()
        .find(|a| a.name.eq_ignore_ascii_case(WINDOWS_ASSET))
        .map(|a| a.download_url)
        .ok_or_else(|| format!("release asset {WINDOWS_ASSET} not found"))
}

#[cfg(windows)]
fn finalize_windows_update(zip_bytes: &[u8]) -> Result<bool, String> {
    let tmp_dir = std::env::temp_dir().join("nyx_update");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).map_err(|e| e.to_string())?;

    let zip_path = tmp_dir.join("nyx.zip");
    std::fs::write(&zip_path, zip_bytes).map_err(|e| e.to_string())?;

    self_update::Extract::from_source(&zip_path)
        .extract_file(&tmp_dir, "nyx.exe")
        .map_err(|e| e.to_string())?;
    let new_exe = tmp_dir.join("nyx.exe");
    if !new_exe.exists() {
        return Err("update archive did not contain nyx.exe".to_string());
    }

    let install_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    spawn_elevated_swap(&new_exe, &install_exe, &tmp_dir)?;
    Ok(true)
}

#[cfg(windows)]
fn spawn_elevated_swap(
    new_exe: &std::path::Path,
    install_exe: &std::path::Path,
    tmp_dir: &std::path::Path,
) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    // Kill the running nyx processes (the service is already stopped) so the exe
    // unlocks, overwrite it, then relaunch via explorer so the new process runs
    // de-elevated rather than inheriting this script's admin token.
    let script = format!(
        "@echo off\r\nchcp 65001 >nul\r\ntaskkill /F /IM nyx.exe >nul 2>&1\r\nset /a n=0\r\n:retry\r\ncopy /Y \"{new}\" \"{inst}\" >nul 2>&1\r\nif not errorlevel 1 goto done\r\nset /a n+=1\r\nif %n% geq 30 goto done\r\ntimeout /t 1 /nobreak >nul\r\ngoto retry\r\n:done\r\nstart \"\" explorer.exe \"{inst}\"\r\n",
        new = new_exe.display(),
        inst = install_exe.display(),
    );
    let bat = tmp_dir.join("nyx_update.bat");
    std::fs::write(&bat, script).map_err(|e| e.to_string())?;

    let ps = format!(
        "Start-Process -FilePath '{}' -Verb RunAs -WindowStyle Hidden",
        bat.display().to_string().replace('\'', "''")
    );
    std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &ps,
        ])
        .creation_flags(0x08000000)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
