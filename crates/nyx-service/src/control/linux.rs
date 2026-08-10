use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use crate::control::Status;
use crate::host::linux::SOCKET_PATH;
use crate::logging;
use crate::protocol::{CoreSpec, PROTOCOL_VERSION, Request, Response};
use crate::{ARG_HOST, ARG_INSTALL, ARG_OWNER, ARG_UNINSTALL, HELPER_FAILURE, is_elevated};

pub const UNIT_NAME: &str = "nyx.service";
const UNIT_DIRS: [&str; 3] = [
    "/etc/systemd/system",
    "/usr/local/lib/systemd/system",
    "/run/systemd/system",
];
const WANTS_DIR: &str = "multi-user.target.wants";

const CONNECT_GRACE: Duration = Duration::from_secs(10);

const UNIT_TEMPLATE: &str = r#"[Unit]
Description=Nyx Service (mihomo core supervisor)
After=network.target NetworkManager.service systemd-networkd.service iwd.service

[Service]
Type=simple
LimitNPROC=500
LimitNOFILE=1000000
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE CAP_SYS_TIME CAP_SYS_PTRACE CAP_DAC_READ_SEARCH CAP_DAC_OVERRIDE CAP_CHOWN CAP_FOWNER
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW CAP_NET_BIND_SERVICE CAP_SYS_TIME CAP_SYS_PTRACE CAP_DAC_READ_SEARCH CAP_DAC_OVERRIDE
Restart=always
RestartSec=2
RuntimeDirectory=nyx
RuntimeDirectoryMode=0755
ExecStart={exe} {arg_host} {arg_owner} {uid}

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
    require_systemd()?;
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

fn require_systemd() -> Result<(), String> {
    if Path::new("/run/systemd/system").is_dir() {
        return Ok(());
    }
    Err(
        "this system does not run systemd, so the Nyx service cannot be installed — switch the \
         core permission mode to direct"
            .into(),
    )
}

pub fn install_here(uid: u32) -> Result<(), String> {
    require_systemd()?;

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let unit = UNIT_TEMPLATE
        .replace("{exe}", &exe.display().to_string())
        .replace("{arg_host}", ARG_HOST)
        .replace("{arg_owner}", ARG_OWNER)
        .replace("{uid}", &uid.to_string());

    remove_unit_files();

    let mut errors = Vec::new();
    for dir in UNIT_DIRS.map(PathBuf::from) {
        match write_unit(&dir, &unit) {
            Ok(()) => {
                logging::log(&format!("installed {UNIT_NAME} in {}", dir.display()));
                if dir.starts_with("/run") {
                    logging::log(
                        "no persistent unit directory was writable — the service will not \
                         survive a reboot",
                    );
                }
                systemctl(&["daemon-reload"])?;
                systemctl(&["restart", UNIT_NAME])?;
                return Ok(());
            }
            Err(e) => errors.push(e),
        }
    }
    Err(errors.join("; "))
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

const SYSTEMCTL_FALLBACKS: [&str; 4] = [
    "/run/current-system/sw/bin/systemctl",
    "/usr/bin/systemctl",
    "/bin/systemctl",
    "/usr/sbin/systemctl",
];

fn systemctl_bin() -> PathBuf {
    which("systemctl")
        .or_else(|| {
            SYSTEMCTL_FALLBACKS
                .iter()
                .map(PathBuf::from)
                .find(|p| p.exists())
        })
        .unwrap_or_else(|| PathBuf::from("systemctl"))
}

fn systemctl(args: &[&str]) -> Result<(), String> {
    let out = Command::new(systemctl_bin())
        .args(args)
        .output()
        .map_err(|e| format!("cannot run systemctl: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(format!("systemctl {} failed: {stderr}", args.join(" ")))
}

const POLKIT_AGENT_UNITS: [&str; 8] = [
    "hyprpolkitagent.service",
    "polkit-gnome-authentication-agent-1.service",
    "plasma-polkit-agent.service",
    "polkit-kde-authentication-agent-1.service",
    "lxqt-policykit-agent.service",
    "xfce-polkit.service",
    "mate-polkit.service",
    "pantheon-agent-polkit.service",
];

const POLKIT_AGENT_BINARIES: [&str; 8] = [
    "hyprpolkitagent",
    "polkit-gnome-authentication-agent-1",
    "polkit-kde-authentication-agent-1",
    "lxqt-policykit-agent",
    "lxpolkit",
    "polkit-mate-authentication-agent-1",
    "xfce-polkit",
    "pantheon-agent-polkit",
];

const AGENT_DIRS: [&str; 6] = [
    "/usr/libexec",
    "/usr/lib/polkit-1",
    "/usr/lib/polkit-gnome",
    "/usr/lib/polkit-kde",
    "/usr/local/libexec",
    "/run/current-system/sw/libexec",
];

const SESSION_VARS: [&str; 5] = [
    "WAYLAND_DISPLAY",
    "DISPLAY",
    "XAUTHORITY",
    "XDG_CURRENT_DESKTOP",
    "XDG_SESSION_TYPE",
];

const AGENT_SETTLE: Duration = Duration::from_millis(1200);

static SPAWNED_AGENT: Mutex<Option<Child>> = Mutex::new(None);

enum Escalation {
    NoAgent,
    Dismissed,
    Denied,
    Failed(String),
}

impl Escalation {
    fn message(self, manual: &str) -> String {
        match self {
            Self::NoAgent => format!(
                "no polkit authentication agent is running in this session, so the password \
                 prompt cannot be shown. Start one (for example `systemctl --user start \
                 hyprpolkitagent`, or enable it in your session config), or run `{manual}` in a \
                 terminal."
            ),
            Self::Dismissed => "the authorisation request was cancelled".to_string(),
            Self::Denied => format!(
                "polkit refused the authorisation. Make sure your user is an administrator \
                 (in the `wheel` or `sudo` group), or run `{manual}` in a terminal."
            ),
            Self::Failed(detail) => detail,
        }
    }
}

/// pkexec blocks on a polkit prompt, so it never runs on an async worker.
async fn run_privileged(exe: &Path, args: &[&str]) -> Result<(), String> {
    let manual = format!("sudo {} {}", exe.display(), args.join(" "));
    if which("pkexec").is_none() {
        return Err(format!(
            "pkexec is not installed, so Nyx cannot ask for administrator rights. Install \
             polkit, or run `{manual}` in a terminal."
        ));
    }

    match pkexec(exe, args).await {
        Ok(()) => Ok(()),
        Err(Escalation::NoAgent) => {
            if !start_polkit_agent().await {
                return Err(Escalation::NoAgent.message(&manual));
            }
            match pkexec(exe, args).await {
                Err(Escalation::NoAgent) => {
                    tokio::time::sleep(AGENT_SETTLE).await;
                    pkexec(exe, args).await.map_err(|e| e.message(&manual))
                }
                other => other.map_err(|e| e.message(&manual)),
            }
        }
        Err(e) => Err(e.message(&manual)),
    }
}

async fn pkexec(exe: &Path, args: &[&str]) -> Result<(), Escalation> {
    let exe = exe.to_path_buf();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let out = tokio::task::spawn_blocking(move || {
        let mut cmd = Command::new("pkexec");
        cmd.arg(&exe).args(&args).stdin(Stdio::null());
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
        cmd.output()
    })
    .await
    .map_err(|e| Escalation::Failed(e.to_string()))?
    .map_err(|e| Escalation::Failed(format!("cannot run pkexec: {e}")))?;

    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    log::warn!(
        "[service] pkexec exited with {:?}: {stderr}",
        out.status.code()
    );
    Err(match out.status.code() {
        // The helper reports its own failures with a private code, so pkexec's
        // 126/127 stay unambiguous.
        Some(HELPER_FAILURE) if !stderr.is_empty() => Escalation::Failed(stderr),
        Some(HELPER_FAILURE) => Escalation::Failed("the privileged helper failed".to_string()),
        Some(126) => Escalation::Dismissed,
        Some(127) if is_missing_agent(&stderr) => Escalation::NoAgent,
        Some(127) if is_denied(&stderr) => Escalation::Denied,
        _ if stderr.is_empty() => Escalation::Failed("the privileged helper failed".to_string()),
        _ => Escalation::Failed(stderr),
    })
}

fn is_missing_agent(stderr: &str) -> bool {
    let s = stderr.to_ascii_lowercase();
    s.contains("authentication agent") || s.contains("/dev/tty")
}

fn is_denied(stderr: &str) -> bool {
    let s = stderr.to_ascii_lowercase();
    s.contains("not authorized")
        || s.contains("incident has been reported")
        || s.contains("authentication failed")
}

/// Best effort: bring up an agent the machine already ships. Returns whether one
/// was started, so the caller only retries when something changed.
async fn start_polkit_agent() -> bool {
    if spawned_agent_alive() {
        return false;
    }
    let started = tokio::task::spawn_blocking(|| {
        export_session_env();
        for unit in POLKIT_AGENT_UNITS {
            let Some(exec) = user_unit_exec_start(unit) else {
                continue;
            };
            if systemctl_user(&["start", unit]).is_ok() && user_unit_active(unit) {
                log::info!("[service] started polkit agent unit {unit}");
                return true;
            }
            if spawn_agent(&exec) {
                return true;
            }
        }
        POLKIT_AGENT_BINARIES
            .iter()
            .filter_map(|bin| find_agent_binary(bin))
            .any(|path| spawn_agent(&[path.display().to_string()]))
    })
    .await
    .unwrap_or(false);

    if started {
        // The agent registers with polkitd asynchronously; pkexec run too soon
        // still sees no agent.
        tokio::time::sleep(AGENT_SETTLE).await;
    }
    started
}

fn spawned_agent_alive() -> bool {
    let Ok(mut guard) = SPAWNED_AGENT.lock() else {
        return false;
    };
    match guard.as_mut().map(|child| child.try_wait()) {
        Some(Ok(None)) => true,
        _ => {
            *guard = None;
            false
        }
    }
}

fn spawn_agent(exec: &[String]) -> bool {
    let Some((program, args)) = exec.split_first() else {
        return false;
    };
    match Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => {
            log::info!("[service] spawned polkit agent {program}");
            if let Ok(mut guard) = SPAWNED_AGENT.lock() {
                *guard = Some(child);
            }
            true
        }
        Err(e) => {
            log::warn!("[service] could not spawn {program}: {e}");
            false
        }
    }
}

/// The agent binary is often reachable only through its unit, so the unit doubles as a lookup table.
fn user_unit_exec_start(unit: &str) -> Option<Vec<String>> {
    let out = Command::new(systemctl_bin())
        .args(["--user", "cat", unit])
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text
        .lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("ExecStart="))?;
    let words: Vec<String> = line
        .trim_start_matches(['-', '@', '+', '!', ':'])
        .split_whitespace()
        .map(String::from)
        .collect();
    (!words.is_empty()).then_some(words)
}

fn user_unit_active(unit: &str) -> bool {
    Command::new(systemctl_bin())
        .args(["--user", "is-active", "--quiet", unit])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Compositors that never import their session into the user manager leave agent
/// units stuck on `ConditionEnvironment`.
fn export_session_env() {
    let mut args = vec!["import-environment"];
    args.extend(
        SESSION_VARS
            .iter()
            .copied()
            .filter(|v| std::env::var_os(v).is_some()),
    );
    if args.len() > 1 {
        let _ = systemctl_user(&args);
    }
}

fn systemctl_user(args: &[&str]) -> Result<(), String> {
    let out = Command::new(systemctl_bin())
        .arg("--user")
        .args(args)
        .output()
        .map_err(|e| format!("cannot run systemctl --user: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
}

fn find_agent_binary(bin: &str) -> Option<PathBuf> {
    which(bin).or_else(|| {
        AGENT_DIRS
            .iter()
            .map(|dir| Path::new(dir).join(bin))
            .find(|p| p.is_file())
    })
}

fn which(bin: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(bin))
            .find(|p| p.is_file())
    })
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
    Command::new(systemctl_bin())
        .args(["is-active", "--quiet", UNIT_NAME])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Why the unit is not up, in one line, for error messages the user actually sees.
fn unit_diagnostics() -> String {
    let out = Command::new(systemctl_bin())
        .args([
            "show",
            "-p",
            "ActiveState",
            "-p",
            "SubState",
            "-p",
            "Result",
            "-p",
            "ExecMainStatus",
            UNIT_NAME,
        ])
        .output();
    match out {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join(", "),
        _ => "unit state unavailable".to_string(),
    }
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
                "the Nyx service did not open {SOCKET_PATH} in time ({})",
                unit_diagnostics()
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
