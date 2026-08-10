#[cfg(unix)]
pub mod linux;
#[cfg(windows)]
pub mod windows;

use std::process::Stdio;
use std::time::Duration;

use tokio::process::Child;

use crate::logging;
use crate::protocol::{CoreSpec, PROTOCOL_VERSION, Request, Response};

/// How long a freshly spawned core is watched before reporting success — a bad
/// binary or config dies well inside this window, so the app gets a real error.
const SETTLE: Duration = Duration::from_millis(400);

/// Grace period for the core to exit on SIGTERM before it is killed.
const STOP_GRACE: Duration = Duration::from_secs(3);

#[derive(Default)]
pub struct CoreManager {
    child: Option<Child>,
}

impl CoreManager {
    pub async fn handle(&mut self, req: Request) -> Response {
        match req {
            Request::StartCore(spec) => match self.start(spec).await {
                Ok(pid) => Response::Started { pid },
                Err(message) => {
                    logging::log(&format!("start failed: {message}"));
                    Response::Error { message }
                }
            },
            Request::StopCore => {
                self.stop().await;
                Response::Ok
            }
            Request::Ping => Response::Pong {
                protocol_version: PROTOCOL_VERSION,
                core_pid: self.live_pid(),
            },
        }
    }

    async fn start(&mut self, spec: CoreSpec) -> Result<u32, String> {
        logging::clean_old(spec.max_log_days);
        self.stop().await;

        if !spec.binary.exists() {
            return Err(format!("core binary not found: {}", spec.binary.display()));
        }
        if !spec.config.exists() {
            return Err(format!("config not found: {}", spec.config.display()));
        }

        let mut cmd = tokio::process::Command::new(&spec.binary);
        cmd.arg("-d")
            .arg(&spec.work_dir)
            .arg("-f")
            .arg(&spec.config)
            .stdin(Stdio::null())
            .stdout(core_output())
            .stderr(core_output())
            .kill_on_drop(true);

        #[cfg(windows)]
        cmd.creation_flags(0x08000000);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("failed to spawn {}: {e}", spec.binary.display()))?;
        let pid = child.id().unwrap_or(0);

        tokio::time::sleep(SETTLE).await;
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!(
                "core exited immediately ({status}); see {}",
                logging::log_dir().join("mihomo.log").display()
            ));
        }

        logging::log(&format!("started core pid={pid} config={:?}", spec.config));
        self.child = Some(child);
        Ok(pid)
    }

    async fn stop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        let pid = child.id().unwrap_or(0);
        request_termination(&mut child, pid);

        let deadline = tokio::time::Instant::now() + STOP_GRACE;
        loop {
            match child.try_wait() {
                Ok(Some(_)) | Err(_) => break,
                Ok(None) if tokio::time::Instant::now() >= deadline => {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    break;
                }
                Ok(None) => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        }
        logging::log(&format!("stopped core pid={pid}"));
    }

    fn live_pid(&mut self) -> Option<u32> {
        let child = self.child.as_mut()?;
        match child.try_wait() {
            Ok(None) => child.id(),
            _ => {
                self.child = None;
                None
            }
        }
    }
}

fn core_output() -> Stdio {
    let dir = logging::log_dir();
    let _ = std::fs::create_dir_all(&dir);
    std::fs::File::create(dir.join("mihomo.log"))
        .map(Stdio::from)
        .unwrap_or_else(|_| Stdio::null())
}

#[cfg(unix)]
fn request_termination(_child: &mut Child, pid: u32) {
    if pid > 0 {
        unsafe { libc::kill(pid as i32, libc::SIGTERM) };
    }
}

#[cfg(not(unix))]
fn request_termination(_child: &mut Child, _pid: u32) {}

pub fn encode(res: &Response) -> Vec<u8> {
    serde_json::to_vec(res).unwrap_or_else(|_| br#"{"status":"ERROR","message":"encode"}"#.to_vec())
}

pub fn decode(bytes: &[u8]) -> Result<Request, String> {
    serde_json::from_slice(bytes).map_err(|e| format!("invalid request: {e}"))
}
