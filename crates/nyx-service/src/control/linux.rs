use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use crate::control::Status;
use crate::host::linux::SOCKET_PATH;
use crate::protocol::{CoreSpec, PROTOCOL_VERSION, Request, Response};
use crate::{ARG_HOST, ARG_INSTALL, ARG_OWNER, ARG_UNINSTALL, is_elevated};

pub const UNIT_NAME: &str = "nyx.service";
const UNIT_DIRS: [&str; 2] = ["/etc/systemd/system", "/usr/local/lib/systemd/system"];
const WANTS_DIR: &str = "multi-user.target.wants";

const CONNECT_GRACE: Duration = Duration::from_secs(10);

const UNIT_TEMPLATE: &str = r#"[Unit]
Description=Nyx Service (mihomo core supervisor)
After=network.target NetworkManager.service systemd-networkd.service iwd.service

[Service]
Type=simple
LimitNPROC=500
LimitNOFILE=1000000
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE CAP_SYS_TIME CAP_SYS_PTRACE CAP_DAC_READ_SEARCH CAP_DAC_OVERRIDE
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE CAP_SYS_TIME CAP_SYS_PTRACE CAP_DAC_READ_SEARCH CAP_DAC_OVERRIDE
Restart=always
RestartSec=2
RuntimeDirectory=nyx
RuntimeDirectoryMode=0755
ExecStart={exe} {arg_host} {arg_owner} {uid}
ExecReload=/bin/kill -HUP $MAINPID

[Install]
WantedBy=multi-user.target
"#;

pub async fn status() -> Result<Status, String> {
    let Some(exec_start) = unit_exec_start() else {
        return Ok(Status::NotInstalled);
    };

    let current = std::env::current_exe().map_err(|e| e.to_string())?;
    if exec_start != current {
        return Ok(Status::Stale {
            reason: format!(
                "the service points at {}, this build runs from {}",
                exec_start.display(),
                current.display()
            ),
        });
    }

    if !unit_is_active() {
        return Ok(Status::Stopped);
    }
    match ping().await {
        Ok(_) => Ok(Status::Running),
        Err(reason) => Ok(Status::Stale { reason }),
    }
}

pub async fn install() -> Result<(), String> {
    if is_elevated() {
        install_here(owner_uid())?;
    } else {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let uid = owner_uid().to_string();
        run_privileged(&exe, &[ARG_INSTALL, ARG_OWNER, &uid]).await?;
    }
    wait_for_socket(CONNECT_GRACE).await?;
    ping().await.map(|_| ())
}

pub async fn uninstall() -> Result<(), String> {
    if unit_exec_start().is_none() {
        return Ok(());
    }
    let _ = stop_core().await;
    if is_elevated() {
        uninstall_here()?;
    } else {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        run_privileged(&exe, &[ARG_UNINSTALL]).await?;
    }
    Ok(())
}

pub async fn start_core(spec: &CoreSpec) -> Result<u32, String> {
    match status().await? {
        Status::NotInstalled => return Err("the Nyx service is not installed".into()),
        Status::Running => {}
        _ => install().await?,
    }
    match request(&Request::StartCore(spec.clone())).await? {
        Response::Started { pid } => Ok(pid),
        Response::Error { message } => Err(message),
        other => Err(format!("unexpected service response: {other:?}")),
    }
}

pub async fn stop_core() -> Result<(), String> {
    if !Path::new(SOCKET_PATH).exists() {
        return Ok(());
    }
    match request(&Request::StopCore).await? {
        Response::Ok => Ok(()),
        Response::Error { message } => Err(message),
        other => Err(format!("unexpected service response: {other:?}")),
    }
}

/// `Ok(core_pid)` — `None` means the host is up but no core is running.
pub async fn ping() -> Result<Option<u32>, String> {
    match request(&Request::Ping).await? {
        Response::Pong {
            protocol_version,
            core_pid,
        } if protocol_version == PROTOCOL_VERSION => Ok(core_pid),
        Response::Pong {
            protocol_version, ..
        } => Err(format!(
            "service speaks protocol v{protocol_version}, this build expects v{PROTOCOL_VERSION}"
        )),
        Response::Error { message } => Err(message),
        other => Err(format!("unexpected service response: {other:?}")),
    }
}

pub fn install_here(uid: u32) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let unit = UNIT_TEMPLATE
        .replace("{exe}", &exe.display().to_string())
        .replace("{arg_host}", ARG_HOST)
        .replace("{arg_owner}", ARG_OWNER)
        .replace("{uid}", &uid.to_string());

    remove_unit_files();

    let mut last_error = String::from("no writable systemd unit directory");
    for dir in UNIT_DIRS.map(PathBuf::from) {
        match write_unit(&dir, &unit) {
            Ok(()) => {
                systemctl(&["daemon-reload"])?;
                systemctl(&["start", UNIT_NAME])?;
                return Ok(());
            }
            Err(e) => last_error = e,
        }
    }
    Err(last_error)
}

/// Writes the unit plus its `.wants` link, so it also starts at boot without
/// `systemctl enable`, which insists on `/etc`.
fn write_unit(dir: &Path, unit: &str) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let path = dir.join(UNIT_NAME);
    std::fs::write(&path, unit).map_err(|e| format!("cannot write {}: {e}", path.display()))?;

    let wants = dir.join(WANTS_DIR);
    std::fs::create_dir_all(&wants)
        .map_err(|e| format!("cannot create {}: {e}", wants.display()))?;
    let link = wants.join(UNIT_NAME);
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(&path, &link)
        .map_err(|e| format!("cannot link {}: {e}", link.display()))
}

pub fn uninstall_here() -> Result<(), String> {
    let _ = systemctl(&["stop", UNIT_NAME]);
    remove_unit_files();
    let _ = systemctl(&["daemon-reload"]);
    Ok(())
}

fn remove_unit_files() {
    for dir in UNIT_DIRS.map(PathBuf::from) {
        let _ = std::fs::remove_file(dir.join(UNIT_NAME));
        let _ = std::fs::remove_file(dir.join(WANTS_DIR).join(UNIT_NAME));
    }
}

fn systemctl(args: &[&str]) -> Result<(), String> {
    let out = std::process::Command::new("systemctl")
        .args(args)
        .output()
        .map_err(|e| format!("cannot run systemctl: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(format!("systemctl {} failed: {stderr}", args.join(" ")))
}

/// pkexec blocks on a polkit prompt, so it never runs on an async worker.
async fn run_privileged(exe: &Path, args: &[&str]) -> Result<(), String> {
    let exe = exe.to_path_buf();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    tokio::task::spawn_blocking(move || {
        let out = std::process::Command::new("pkexec")
            .arg(&exe)
            .args(&args)
            .output()
            .map_err(|e| format!("cannot run pkexec: {e}"))?;
        if out.status.success() {
            return Ok(());
        }
        // 126 = authorisation dialog dismissed, 127 = not authorised.
        if matches!(out.status.code(), Some(126) | Some(127)) {
            return Err("the authorisation request was cancelled".to_string());
        }
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            "the privileged helper failed".to_string()
        } else {
            stderr
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

fn unit_exec_start() -> Option<PathBuf> {
    let text = UNIT_DIRS
        .iter()
        .find_map(|dir| std::fs::read_to_string(Path::new(dir).join(UNIT_NAME)).ok())?;
    let line = text
        .lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("ExecStart="))?;
    line.split_whitespace().next().map(PathBuf::from)
}

fn unit_is_active() -> bool {
    std::process::Command::new("systemctl")
        .args(["is-active", "--quiet", UNIT_NAME])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn owner_uid() -> u32 {
    // Under pkexec the real user is in PKEXEC_UID; otherwise we are that user.
    std::env::var("PKEXEC_UID")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| unsafe { libc::getuid() })
}

async fn wait_for_socket(grace: Duration) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + grace;
    loop {
        if UnixStream::connect(SOCKET_PATH).await.is_ok() {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "the Nyx service did not open {SOCKET_PATH} in time"
            ));
        }
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}

async fn request(req: &Request) -> Result<Response, String> {
    let mut stream = UnixStream::connect(SOCKET_PATH)
        .await
        .map_err(|e| format!("cannot reach the Nyx service: {e}"))?;

    let payload = serde_json::to_vec(req).map_err(|e| e.to_string())?;
    stream
        .write_all(&payload)
        .await
        .map_err(|e| format!("failed to send request: {e}"))?;
    // The host reads one request per connection; half-close so it stops waiting.
    let _ = stream.shutdown().await;

    let mut buf = Vec::new();
    stream
        .read_to_end(&mut buf)
        .await
        .map_err(|e| format!("failed to read response: {e}"))?;
    serde_json::from_slice(&buf).map_err(|e| format!("invalid service response: {e}"))
}
