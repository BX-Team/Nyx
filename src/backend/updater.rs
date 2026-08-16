use serde::{Deserialize, Serialize};

const REPO_OWNER: &str = "BX-Team";
const REPO_NAME: &str = "Nyx";
const BIN_NAME: &str = "nyx";

#[cfg(windows)]
const WINDOWS_ASSET: &str = "Nyx-x86_64-windows.zip";
#[cfg(not(windows))]
const LINUX_TARGET: &str = "x86_64-linux";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub changelog: String,
}

fn latest_release() -> Result<self_update::update::Release, String> {
    self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        .current_version(self_update::cargo_crate_version!())
        .build()
        .map_err(|e| e.to_string())?
        .get_latest_release()
        .map_err(|e| e.to_string())
}

pub async fn check() -> Result<Option<UpdateInfo>, String> {
    tokio::task::spawn_blocking(|| {
        let latest = latest_release()?;
        let current = self_update::cargo_crate_version!();
        let newer = self_update::version::bump_is_greater(current, &latest.version)
            .map_err(|e| e.to_string())?;
        if !newer {
            return Ok(None);
        }
        Ok(Some(UpdateInfo {
            version: latest.version,
            changelog: latest.body.unwrap_or_default(),
        }))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Installs the newest release. `true` means an elevated helper will relaunch
/// (Windows, where the running exe is locked); `false` means the caller should.
pub async fn download_and_install() -> Result<bool, String> {
    #[cfg(windows)]
    {
        windows_update().await
    }

    #[cfg(not(windows))]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        if exe.starts_with("/nix/store") || std::path::Path::new("/etc/NIXOS").exists() {
            return Err(
                "self-update is not available on NixOS — update the Nyx flake input / package instead"
                    .to_string(),
            );
        }
        if !path_writable(&exe) {
            return Err(format!(
                "no write access to {} — update Nyx via your package manager",
                exe.display()
            ));
        }
        tokio::task::spawn_blocking(|| {
            let tag = format!("v{}", latest_release()?.version);
            self_update::backends::github::Update::configure()
                .repo_owner(REPO_OWNER)
                .repo_name(REPO_NAME)
                .target(LINUX_TARGET)
                .bin_name(BIN_NAME)
                .current_version(self_update::cargo_crate_version!())
                .target_version_tag(&tag)
                .no_confirm(true)
                .show_output(false)
                .show_download_progress(false)
                .build()
                .map_err(|e| e.to_string())?
                .update()
                .map_err(|e| e.to_string())?;
            Ok::<(), String>(())
        })
        .await
        .map_err(|e| e.to_string())??;

        let _ = crate::backend::core::stop().await;
        crate::backend::sysproxy::clear();
        if let Err(e) = nyx_service::restart_service().await {
            log::warn!("[updater] could not restart the service after the update: {e}");
        }
        Ok(false)
    }
}

#[cfg(not(windows))]
fn path_writable(path: &std::path::Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::CString::new(path.as_os_str().as_bytes())
        .map(|c| unsafe { libc::access(c.as_ptr(), libc::W_OK) } == 0)
        .unwrap_or(false)
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
        .header(reqwest::header::ACCEPT, "application/octet-stream")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    if !bytes.starts_with(b"PK") {
        return Err(format!(
            "update download is not a zip archive ({} bytes)",
            bytes.len()
        ));
    }

    let _ = crate::backend::core::stop().await;
    crate::backend::sysproxy::clear();

    tokio::task::spawn_blocking(move || finalize_windows_update(&bytes))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(windows)]
fn windows_asset_url() -> Result<String, String> {
    latest_release()?
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

    let script = format!(
        "@echo off\r\nchcp 65001 >nul\r\nsc stop \"{svc}\" >nul 2>&1\r\ntaskkill /F /PID {pid} >nul 2>&1\r\nset /a n=0\r\n:retry\r\ncopy /Y \"{new}\" \"{inst}\" >nul 2>&1\r\nif not errorlevel 1 goto done\r\nset /a n+=1\r\nif %n% geq 30 goto done\r\ntimeout /t 1 /nobreak >nul\r\ngoto retry\r\n:done\r\nsc start \"{svc}\" >nul 2>&1\r\nstart \"\" explorer.exe \"{inst}\"\r\n",
        svc = nyx_service::SERVICE_NAME,
        pid = std::process::id(),
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
