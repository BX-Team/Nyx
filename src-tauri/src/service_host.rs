#[cfg(windows)]
pub mod imp {
    use std::ffi::{c_void, OsString};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::mpsc;
    use std::time::Duration;
    use serde::{Deserialize, Serialize};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
    use tokio::process::Child;
    use windows::core::{BOOL, PCWSTR};
    use windows::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
    use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
    use windows_service::define_windows_service;
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::service_dispatcher;

    const SERVICE_NAME: &str = "Nyx Service";
    pub const IPC_PIPE_NAME: &str = r"\\.\pipe\nyx_mihomo_ipc";

    fn build_pipe_security_attributes() -> Option<SECURITY_ATTRIBUTES> {
        let sddl: Vec<u16> = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGWGX;;;AU)\0"
            .encode_utf16()
            .collect();
        let mut psd = PSECURITY_DESCRIPTOR::default();
        unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                PCWSTR::from_raw(sddl.as_ptr()),
                1,
                &mut psd as *mut _,
                None,
            )
            .ok()?;
        }
        Some(SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: psd.0,
            bInheritHandle: BOOL(0),
        })
    }

    fn create_pipe_server(first: bool) -> std::io::Result<NamedPipeServer> {
        let mut opts = ServerOptions::new();
        opts.first_pipe_instance(first);
        match build_pipe_security_attributes() {
            Some(mut sa) => unsafe {
                opts.create_with_security_attributes_raw(
                    IPC_PIPE_NAME,
                    &mut sa as *mut SECURITY_ATTRIBUTES as *mut c_void,
                )
            },
            None => opts.create(IPC_PIPE_NAME),
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "action", rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum IpcRequest {
        StartCore {
            binary: String,
            work_dir: String,
            config: String,
            #[serde(default = "default_max_log_days")]
            max_log_days: u32,
        },
        StopCore,
        Ping,
    }

    fn default_max_log_days() -> u32 {
        7
    }

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "status", rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum IpcResponse {
        Ok,
        Error { message: String },
        Pong,
    }

    pub fn log_to_file(msg: &str) {
        let log_dir = Path::new("C:\\ProgramData\\Nyx");
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let log_path = log_dir.join(format!("{}.log", today));
        let _ = std::fs::create_dir_all(log_dir);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log_path) {
            use std::time::SystemTime;
            let ts = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = writeln!(f, "[{ts}] {msg}");
        }
    }

    fn clean_old_logs(max_days: u32) {
        if max_days == 0 {
            return;
        }

        let today = chrono::Local::now().date_naive();
        let cutoff = match today.checked_sub_signed(chrono::Duration::days(max_days as i64 - 1)) {
            Some(d) => d,
            None => return,
        };

        let log_dir = Path::new("C:\\ProgramData\\Nyx");
        let Ok(entries) = std::fs::read_dir(log_dir) else {
            return;
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("log") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let Ok(file_date) = chrono::NaiveDate::parse_from_str(stem, "%Y-%m-%d") else {
                continue;
            };
            if file_date < cutoff {
                let _ = std::fs::remove_file(&path);
            }
        }
    }

    define_windows_service!(ffi_service_main, service_main);

    pub fn maybe_run_as_service_from_args() -> Option<i32> {
        if !std::env::args().any(|a| a == "--nyx-service") {
            return None;
        }

        if let Err(e) = service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
            log_to_file(&format!("failed to start service dispatcher: {e}"));
            return Some(1);
        }

        Some(0)
    }

    fn service_main(_arguments: Vec<OsString>) {
        if let Err(e) = run_service() {
            log_to_file(&format!("service runtime error: {e}"));
        }
    }

    fn run_service() -> windows_service::Result<()> {
        log_to_file("service_main: initializing 24/7 service");

        let (stop_tx, stop_rx) = mpsc::channel::<()>();

        let status_handle = service_control_handler::register(SERVICE_NAME, move |control_event| {
            match control_event {
                ServiceControl::Stop => {
                    let _ = stop_tx.send(());
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        })?;

        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::from_secs(0),
            process_id: None,
        })?;

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime");

        rt.block_on(async move {
            let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);
            let (child_tx, child_rx) = tokio::sync::mpsc::channel::<IpcRequest>(10);

            let manager_handle = tokio::spawn(async move {
                let mut current_child: Option<Child> = None;
                let mut rx = child_rx;

                while let Some(req) = rx.recv().await {
                    match req {
                        IpcRequest::StartCore { binary, work_dir, config, max_log_days } => {
                            let _ = tokio::task::spawn_blocking(move || {
                                clean_old_logs(max_log_days);
                            });

                            if let Some(mut child) = current_child.take() {
                                let _ = child.kill().await;
                            }
                            
                            let mut cmd = tokio::process::Command::new(&binary);
                            cmd.arg("-d")
                                .arg(if work_dir.is_empty() {
                                    PathBuf::from(&config).parent().unwrap_or(Path::new("")).to_string_lossy().into_owned()
                                } else {
                                    work_dir.clone()
                                })
                                .arg("-f")
                                .arg(&config)
                                .stdin(std::process::Stdio::null())
                                .stdout(std::process::Stdio::null())
                                .stderr(std::process::Stdio::null());

                            cmd.creation_flags(0x08000000); 

                            match cmd.spawn() {
                                Ok(child) => {
                                    log_to_file(&format!("service manager: spawned mihomo pid={:?}", child.id()));
                                    current_child = Some(child);
                                }
                                Err(e) => {
                                    log_to_file(&format!("service manager: failed to spawn mihomo: {}", e));
                                }
                            }
                        }
                        IpcRequest::StopCore => {
                            if let Some(mut child) = current_child.take() {
                                let pid = child.id().unwrap_or(0);
                                log_to_file(&format!("service manager: stopping mihomo pid={}", pid));
                                let _ = child.kill().await;

                                if pid > 0 {
                                    use std::os::windows::process::CommandExt;
                                    let _ = std::process::Command::new("taskkill")
                                        .args(["/F", "/PID", &pid.to_string()])
                                        .creation_flags(0x08000000)
                                        .output();
                                }
                            }
                        }
                        _ => {}
                    }
                }

                if let Some(mut child) = current_child.take() {
                    let _ = child.kill().await;
                }
            });

            let server_task = tokio::spawn(async move {
                loop {
                    let mut server = match create_pipe_server(true) {
                        Ok(s) => s,
                        Err(_) => match create_pipe_server(false) {
                            Ok(s) => s,
                            Err(e) => {
                                log_to_file(&format!("failed to create pipe: {}", e));
                                tokio::time::sleep(Duration::from_secs(1)).await;
                                continue;
                            }
                        },
                    };

                    if let Err(e) = server.connect().await {
                        log_to_file(&format!("pipe connect error: {}", e));
                        continue;
                    }

                    let mut buf = vec![0u8; 8192];
                    match server.read(&mut buf).await {
                        Ok(n) if n > 0 => {
                            let msg = String::from_utf8_lossy(&buf[..n]);
                            if let Ok(req) = serde_json::from_str::<IpcRequest>(&msg) {
                                log_to_file(&format!("Got IPC request: {:?}", req));
                                match req {
                                    IpcRequest::StartCore { .. } | IpcRequest::StopCore => {
                                        let _ = child_tx.send(req).await;
                                        let res = serde_json::to_string(&IpcResponse::Ok).unwrap();
                                        let _ = server.write_all(res.as_bytes()).await;
                                    }
                                    IpcRequest::Ping => {
                                        let res = serde_json::to_string(&IpcResponse::Pong).unwrap();
                                        let _ = server.write_all(res.as_bytes()).await;
                                    }
                                }
                            } else {
                                let res = serde_json::to_string(&IpcResponse::Error {
                                    message: "Invalid request payload".to_string(),
                                }).unwrap();
                                let _ = server.write_all(res.as_bytes()).await;
                            }
                        }
                        _ => {}
                    }
                }
            });

            let _ = tokio::task::spawn_blocking(move || {
                let _ = stop_rx.recv();
                let _ = shutdown_tx.blocking_send(());
            }).await;

            let _ = shutdown_rx.recv().await;
            log_to_file("service stopping...");
            server_task.abort();
            manager_handle.abort();
        });

        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::from_secs(0),
            process_id: None,
        })?;

        log_to_file("service successfully stopped");
        Ok(())
    }
}

#[cfg(windows)]
pub use imp::{maybe_run_as_service_from_args, IpcRequest, IpcResponse, IPC_PIPE_NAME};

#[cfg(not(windows))]
pub fn maybe_run_as_service_from_args() -> Option<i32> {
    None
}
