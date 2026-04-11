#[cfg(windows)]
mod imp {
    use std::ffi::OsString;
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::time::Duration;
    use windows_service::define_windows_service;
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
    use windows_service::service_dispatcher;

    const SERVICE_NAME: &str = "NyxMihomo";

    fn log_to_file(msg: &str) {
        let log_dir = std::path::Path::new("C:\\ProgramData\\Nyx");
        let log_path = log_dir.join("service.log");
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

    fn parse_arg_value(flag: &str) -> Option<String> {
        let args: Vec<String> = std::env::args().collect();
        let mut i = 0usize;
        while i < args.len() {
            if args[i] == flag {
                return args.get(i + 1).cloned();
            }
            i += 1;
        }
        None
    }

    fn kill_by_pid(pid: u32) {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .creation_flags(0x08000000) 
            .output();
    }

    fn run_service() -> windows_service::Result<()> {
        let core_binary = parse_arg_value("--core").map(PathBuf::from).unwrap_or_default();
        let work_dir = parse_arg_value("--work-dir").map(PathBuf::from).unwrap_or_default();
        let config = parse_arg_value("--config").map(PathBuf::from).unwrap_or_default();

        log_to_file(&format!(
            "service_main: core={} config={}",
            core_binary.display(),
            config.display()
        ));

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let stop_tx_ctl = stop_tx.clone();
        let stop_tx_crash = stop_tx;

        let status_handle = service_control_handler::register(SERVICE_NAME, move |control_event| {
            match control_event {
                ServiceControl::Stop => {
                    let _ = stop_tx_ctl.send(());
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        })?;

        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::StartPending,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 1,
            wait_hint: Duration::from_secs(30),
            process_id: None,
        })?;

        if core_binary.as_os_str().is_empty() || config.as_os_str().is_empty() {
            log_to_file("service_main: missing core binary or config path, stopping");
            status_handle.set_service_status(ServiceStatus {
                service_type: ServiceType::OWN_PROCESS,
                current_state: ServiceState::Stopped,
                controls_accepted: ServiceControlAccept::empty(),
                exit_code: ServiceExitCode::Win32(87), 
                checkpoint: 0,
                wait_hint: Duration::from_secs(0),
                process_id: None,
            })?;
            return Ok(());
        }

        let mut cmd = std::process::Command::new(&core_binary);
        cmd.arg("-d")
            .arg(if work_dir.as_os_str().is_empty() {
                config.parent().map(PathBuf::from).unwrap_or_default()
            } else {
                work_dir
            })
            .arg("-f")
            .arg(&config)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); 

        let child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                log_to_file(&format!("service_main: failed to spawn mihomo: {e}"));
                status_handle.set_service_status(ServiceStatus {
                    service_type: ServiceType::OWN_PROCESS,
                    current_state: ServiceState::Stopped,
                    controls_accepted: ServiceControlAccept::empty(),
                    exit_code: ServiceExitCode::Win32(2), 
                    checkpoint: 0,
                    wait_hint: Duration::from_secs(0),
                    process_id: None,
                })?;
                return Ok(());
            }
        };

        let child_pid = child.id();

        std::thread::spawn(move || {
            let mut child = child;
            match child.wait() {
                Ok(status) => log_to_file(&format!("service_main: mihomo exited with {status}")),
                Err(e) => log_to_file(&format!("service_main: mihomo wait() error: {e}")),
            }
            let _ = stop_tx_crash.send(());
        });

        log_to_file(&format!("service_main: mihomo spawned (pid={child_pid}), reporting Running"));

        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::from_secs(0),
            process_id: None,
        })?;

        let _ = stop_rx.recv();
        log_to_file("service_main: stop signal received, terminating mihomo");

        kill_by_pid(child_pid);

        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::from_secs(0),
            process_id: None,
        })?;

        log_to_file("service_main: stopped cleanly");
        Ok(())
    }
}

#[cfg(windows)]
pub use imp::maybe_run_as_service_from_args;

#[cfg(not(windows))]
pub fn maybe_run_as_service_from_args() -> Option<i32> {
    None
}
