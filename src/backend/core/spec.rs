use std::path::PathBuf;

use crate::backend::core::{CoreError, FailureKind};
use crate::backend::{dirs, manager};

/// Resolved and validated up front, so a failure names the actual cause.
#[derive(Debug, Clone)]
pub struct StartSpec {
    pub binary: PathBuf,
    pub work_dir: PathBuf,
    pub config: PathBuf,
    pub controller: String,
    pub secret: Option<String>,
    pub max_log_days: u32,
}

pub async fn build() -> Result<StartSpec, CoreError> {
    let app_cfg = read_app_config().await;
    let binary = resolve_binary(&app_cfg).await?;

    let controller = manager::rebuild_config()
        .await
        .map_err(|e| CoreError::new(FailureKind::ConfigInvalid, e.to_string()))?;

    let config = mihomo_rs::ConfigManager::with_home(dirs::data_dir())
        .map_err(|e| CoreError::new(FailureKind::Other, e.to_string()))?
        .get_current_path()
        .await
        .map_err(|e| CoreError::new(FailureKind::Other, e.to_string()))?;

    let work_dir = config
        .parent()
        .map(PathBuf::from)
        .ok_or_else(|| CoreError::new(FailureKind::Other, "config has no parent directory"))?;

    validate(&binary, &work_dir, &config).await?;

    Ok(StartSpec {
        secret: read_secret(&config).await,
        max_log_days: app_cfg
            .get("maxLogDays")
            .and_then(|v| v.as_u64())
            .unwrap_or(7) as u32,
        binary,
        work_dir,
        config,
        controller,
    })
}

async fn read_app_config() -> serde_yaml::Value {
    tokio::fs::read_to_string(dirs::app_config_path())
        .await
        .ok()
        .and_then(|s| serde_yaml::from_str(&s).ok())
        .unwrap_or(serde_yaml::Value::Null)
}

async fn resolve_binary(app_cfg: &serde_yaml::Value) -> Result<PathBuf, CoreError> {
    let core = app_cfg
        .get("core")
        .and_then(|v| v.as_str())
        .unwrap_or("mihomo");

    if core == "system" {
        let path = app_cfg
            .get("systemCorePath")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                CoreError::new(
                    FailureKind::CoreMissing,
                    "no system core path is configured",
                )
            })?;
        let path = PathBuf::from(path);
        if !path.exists() {
            return Err(CoreError::new(
                FailureKind::CoreMissing,
                format!("{} does not exist", path.display()),
            ));
        }
        return Ok(path);
    }

    manager::ensure_core_installed(core)
        .await
        .map_err(|e| CoreError::new(FailureKind::CoreMissing, e.to_string()))?;

    mihomo_rs::VersionManager::with_home(dirs::data_dir())
        .map_err(|e| CoreError::new(FailureKind::CoreMissing, e.to_string()))?
        .get_binary_path(None)
        .await
        .map_err(|e| CoreError::new(FailureKind::CoreMissing, e.to_string()))
}

/// `mihomo -t` parses the merged config without binding anything, so a broken
/// profile is reported here instead of as a silent exit after spawn.
async fn validate(
    binary: &std::path::Path,
    work_dir: &std::path::Path,
    config: &std::path::Path,
) -> Result<(), CoreError> {
    let out = tokio::process::Command::new(binary)
        .arg("-t")
        .arg("-d")
        .arg(work_dir)
        .arg("-f")
        .arg(config)
        .output()
        .await;

    let Ok(out) = out else {
        // Could not even run the binary; the spawn below will report why.
        return Ok(());
    };
    if out.status.success() {
        return Ok(());
    }

    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // A core build without `-t` must not be treated as a broken profile.
    if text.contains("flag provided but not defined") {
        log::warn!("[core] this core does not support -t, skipping config validation");
        return Ok(());
    }

    Err(CoreError::new(FailureKind::ConfigInvalid, summarize(&text)))
}

/// mihomo prints a banner first; the tail holds the offending key.
fn summarize(text: &str) -> String {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let tail = lines
        .iter()
        .rev()
        .take(3)
        .rev()
        .copied()
        .collect::<Vec<_>>();
    if tail.is_empty() {
        "the configuration was rejected by the core".to_string()
    } else {
        tail.join("; ")
    }
}

async fn read_secret(config: &std::path::Path) -> Option<String> {
    let content = tokio::fs::read_to_string(config).await.ok()?;
    let val: serde_yaml::Value = serde_yaml::from_str(&content).ok()?;
    val.get("secret")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}
