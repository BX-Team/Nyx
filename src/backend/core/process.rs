use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::time::Duration;

use once_cell::sync::Lazy;
use parking_lot::Mutex;

use crate::backend::core::spec::StartSpec;
use crate::backend::core::{CoreError, FailureKind};
use crate::backend::dirs;

/// The core we spawned ourselves (unprivileged fallback backend).
static CHILD: Lazy<Mutex<Option<Child>>> = Lazy::new(|| Mutex::new(None));

const SETTLE: Duration = Duration::from_millis(400);
const STOP_GRACE: Duration = Duration::from_secs(3);

fn pid_file() -> PathBuf {
    dirs::data_dir().join("mihomo.pid")
}

pub async fn spawn(spec: &StartSpec) -> Result<u32, CoreError> {
    stop().await;

    let mut cmd = std::process::Command::new(&spec.binary);
    cmd.arg("-d")
        .arg(&spec.work_dir)
        .arg("-f")
        .arg(&spec.config)
        .stdin(Stdio::null())
        .stdout(core_log())
        .stderr(core_log());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }

    let child = cmd.spawn().map_err(|e| {
        CoreError::new(
            FailureKind::CoreMissing,
            format!("failed to run {}: {e}", spec.binary.display()),
        )
    })?;
    let pid = child.id();
    *CHILD.lock() = Some(child);
    write_pid(pid).await;

    tokio::time::sleep(SETTLE).await;
    if let Some(status) = exit_status() {
        CHILD.lock().take();
        return Err(CoreError::new(
            FailureKind::ConfigInvalid,
            format!(
                "the core exited immediately ({status}); see {}",
                core_log_path().display()
            ),
        ));
    }

    Ok(pid)
}

pub fn alive() -> bool {
    let mut guard = CHILD.lock();
    match guard.as_mut() {
        Some(child) => match child.try_wait() {
            Ok(None) => true,
            _ => {
                guard.take();
                false
            }
        },
        None => false,
    }
}

fn exit_status() -> Option<std::process::ExitStatus> {
    CHILD
        .lock()
        .as_mut()
        .and_then(|child| child.try_wait().ok().flatten())
}

pub async fn stop() {
    // Bind first: an `if let` scrutinee would hold the guard across the awaits.
    let owned = CHILD.lock().take();
    if let Some(mut child) = owned {
        terminate(child.id());
        wait_for_exit(&mut child).await;
    } else if let Some(pid) = read_pid().await {
        // A core left behind by a previous run of the app.
        if owned_by_us(pid) {
            terminate(pid);
        }
    }
    let _ = tokio::fs::remove_file(pid_file()).await;
}

async fn wait_for_exit(child: &mut Child) {
    let deadline = tokio::time::Instant::now() + STOP_GRACE;
    loop {
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => return,
            Ok(None) if tokio::time::Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
            Ok(None) => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    }
}

#[cfg(windows)]
fn terminate(pid: u32) {
    use std::os::windows::process::CommandExt;
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(0x08000000)
        .output();
}

#[cfg(not(windows))]
fn terminate(pid: u32) {
    unsafe { libc::kill(pid as i32, libc::SIGTERM) };
}

/// Guards against killing an unrelated process that reused the recorded pid.
#[cfg(target_os = "linux")]
fn owned_by_us(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .map(|comm| comm.trim().contains("mihomo"))
        .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn owned_by_us(_pid: u32) -> bool {
    true
}

async fn write_pid(pid: u32) {
    let path = pid_file();
    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let _ = tokio::fs::write(&path, pid.to_string()).await;
}

async fn read_pid() -> Option<u32> {
    tokio::fs::read_to_string(pid_file())
        .await
        .ok()?
        .trim()
        .parse()
        .ok()
}

fn core_log_path() -> PathBuf {
    dirs::log_dir().join("mihomo.log")
}

fn core_log() -> Stdio {
    let path = core_log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::File::create(&path)
        .map(Stdio::from)
        .unwrap_or_else(|_| Stdio::null())
}
