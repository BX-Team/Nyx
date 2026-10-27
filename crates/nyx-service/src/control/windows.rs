use std::ffi::OsString;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::ClientOptions;
use windows_service::service::{
    ServiceAccess, ServiceAction, ServiceActionType, ServiceErrorControl, ServiceFailureActions,
    ServiceFailureResetPeriod, ServiceInfo, ServiceStartType, ServiceState, ServiceType,
};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

use crate::control::Status;
use crate::host::windows::PIPE_NAME;
use crate::protocol::{CoreSpec, PROTOCOL_VERSION, Request, Response};
use crate::{
    ARG_CONTROL, ARG_HOST, ARG_INSTALL, ARG_OWNER, ARG_UNINSTALL, HELPER_FAILURE,
    SERVICE_DISPLAY_NAME, SERVICE_NAME, is_elevated, logging,
};

const ERR_NO_SERVICE: i32 = 1060;
const CONNECT_GRACE: Duration = Duration::from_secs(10);
const STATE_TIMEOUT: Duration = Duration::from_secs(15);
/// How long a host the SCM reports running gets to answer, e.g. right after a recovery restart.
const SETTLE_GRACE: Duration = Duration::from_secs(3);

pub async fn status() -> Result<Status, String> {
    let Some(state) = query_state()? else {
        return Ok(Status::NotInstalled);
    };
    let current = std::env::current_exe().map_err(|e| e.to_string())?;
    if let Some(registered) = service_exe()
        && !same_path(&registered, &current)
    {
        return Ok(Status::Stale {
            reason: format!(
                "the service points at {}, this build runs from {}",
                registered.display(),
                current.display()
            ),
        });
    }
    if state != ServiceState::Running {
        return Ok(Status::Stopped);
    }
    match ping_within(SETTLE_GRACE).await {
        Ok(_) => Ok(Status::Running),
        Err(reason) => Ok(Status::Stale { reason }),
    }
}

/// The executable the service is registered with, cut out of its command line.
fn service_exe() -> Option<std::path::PathBuf> {
    let manager = open_manager(ServiceManagerAccess::CONNECT).ok()?;
    let service = manager
        .open_service(SERVICE_NAME, ServiceAccess::QUERY_CONFIG)
        .ok()?;
    let line = service.query_config().ok()?.executable_path;
    let line = line.to_string_lossy();
    let line = line.trim();
    let exe = match line.strip_prefix('"') {
        Some(rest) => rest.split('"').next().unwrap_or(rest),
        None => line.split(" --").next().unwrap_or(line),
    };
    Some(std::path::PathBuf::from(exe.trim()))
}

fn same_path(a: &std::path::Path, b: &std::path::Path) -> bool {
    let canon = |p: &std::path::Path| {
        std::fs::canonicalize(p)
            .unwrap_or_else(|_| p.to_path_buf())
            .to_string_lossy()
            .to_lowercase()
    };
    canon(a) == canon(b)
}

pub async fn install() -> Result<(), String> {
    if is_elevated() {
        install_here()?;
    } else {
        elevate(&[ARG_INSTALL, ARG_OWNER, &current_user_sid()])?;
    }
    wait_for_state(true).await?;
    ping().await.map(|_| ())
}

pub async fn uninstall() -> Result<(), String> {
    if query_state()?.is_none() {
        return Ok(());
    }
    let _ = stop_core().await;
    if is_elevated() {
        uninstall_here()?;
    } else {
        elevate(&[ARG_UNINSTALL])?;
    }
    Ok(())
}

pub async fn start_service() -> Result<(), String> {
    control("start")?;
    wait_for_state(true).await?;
    ping().await.map(|_| ())
}

pub async fn stop_service() -> Result<(), String> {
    if query_state()?.is_none() {
        return Ok(());
    }
    control("stop")?;
    wait_for_state(false).await
}

pub async fn restart_service() -> Result<(), String> {
    control("restart")?;
    wait_for_state(true).await?;
    ping().await.map(|_| ())
}

fn control(action: &'static str) -> Result<(), String> {
    if query_state()?.is_none() {
        return Err("the Nyx service is not installed".into());
    }
    if is_elevated() {
        return control_here(action);
    }
    elevate(&[ARG_CONTROL, action])
}

pub fn control_here(action: &str) -> Result<(), String> {
    let manager = open_manager(ServiceManagerAccess::CONNECT)?;
    let service = manager
        .open_service(
            SERVICE_NAME,
            ServiceAccess::START | ServiceAccess::STOP | ServiceAccess::QUERY_STATUS,
        )
        .map_err(|e| format!("failed to open service: {e}"))?;

    let stop = |service: &windows_service::service::Service| -> Result<(), String> {
        let running = service
            .query_status()
            .map(|s| s.current_state != ServiceState::Stopped)
            .unwrap_or(false);
        if !running {
            return Ok(());
        }
        service
            .stop()
            .map(|_| ())
            .map_err(|e| format!("failed to stop service: {e}"))
    };

    match action {
        "start" => start_if_stopped(&service),
        "stop" => stop(&service),
        "restart" => {
            stop(&service)?;
            wait_for_stopped_blocking(&service);
            start_if_stopped(&service)
        }
        other => Err(format!("unknown service action {other:?}")),
    }
}

/// The elevated helper is a plain process, so the wait stays synchronous.
fn wait_for_stopped_blocking(service: &windows_service::service::Service) {
    for _ in 0..50 {
        match service.query_status() {
            Ok(s) if s.current_state == ServiceState::Stopped => return,
            Err(_) => return,
            _ => std::thread::sleep(Duration::from_millis(200)),
        }
    }
}

pub async fn start_core(spec: &CoreSpec) -> Result<u32, String> {
    match status().await? {
        Status::NotInstalled => return Err("the Nyx service is not installed".into()),
        Status::Running => {}
        // Registered for this exe, just not running: starting it is enough.
        Status::Stopped => start_service().await?,
        Status::Stale { .. } => install().await?,
    }
    match request(&Request::StartCore(spec.clone()), CONNECT_GRACE).await? {
        Response::Started { pid } => Ok(pid),
        Response::Error { message } => Err(message),
        other => Err(format!("unexpected service response: {other:?}")),
    }
}

pub async fn stop_core() -> Result<(), String> {
    if query_state()? != Some(ServiceState::Running) {
        return Ok(());
    }
    match request(&Request::StopCore, Duration::ZERO).await? {
        Response::Ok => Ok(()),
        Response::Error { message } => Err(message),
        other => Err(format!("unexpected service response: {other:?}")),
    }
}

/// `Ok(core_pid)` — `None` means the host is up but no core is running.
pub async fn ping() -> Result<Option<u32>, String> {
    ping_within(Duration::ZERO).await
}

async fn ping_within(grace: Duration) -> Result<Option<u32>, String> {
    match request(&Request::Ping, grace).await? {
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

pub fn install_here() -> Result<(), String> {
    let manager =
        open_manager(ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE)?;
    let info = service_info();

    let access = ServiceAccess::CHANGE_CONFIG
        | ServiceAccess::START
        | ServiceAccess::STOP
        | ServiceAccess::QUERY_STATUS;
    let service = match manager.open_service(SERVICE_NAME, access) {
        Ok(service) => {
            service
                .change_config(&info)
                .map_err(|e| format!("failed to reconfigure service: {e}"))?;
            // A host from the previous build keeps running the old exe until restarted.
            if let Ok(status) = service.query_status()
                && status.current_state != ServiceState::Stopped
            {
                let _ = service.stop();
                wait_for_stopped_blocking(&service);
            }
            service
        }
        Err(e) if is_missing_service(&e) => {
            let service = manager
                .create_service(&info, access)
                .map_err(|e| format!("failed to create service: {e}"))?;
            let _ = service.set_description("Runs the mihomo core for Nyx");
            service
        }
        Err(e) => return Err(format!("failed to open service: {e}")),
    };
    set_recovery(&service);
    start_if_stopped(&service)
}

/// Brings a crashed host back like systemd's `Restart=always` does on Linux.
fn set_recovery(service: &windows_service::service::Service) {
    let restart = ServiceAction {
        action_type: ServiceActionType::Restart,
        delay: Duration::from_secs(2),
    };
    let actions = ServiceFailureActions {
        reset_period: ServiceFailureResetPeriod::After(Duration::from_secs(24 * 60 * 60)),
        reboot_msg: None,
        command: None,
        actions: Some(vec![restart.clone(), restart.clone(), restart]),
    };
    if let Err(e) = service.update_failure_actions(actions) {
        logging::log(&format!("cannot set the service recovery actions: {e}"));
    }
    let _ = service.set_failure_actions_on_non_crash_failures(true);
}

pub fn uninstall_here() -> Result<(), String> {
    let manager = open_manager(ServiceManagerAccess::CONNECT)?;
    let service = match manager.open_service(
        SERVICE_NAME,
        ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS,
    ) {
        Ok(s) => s,
        Err(e) if is_missing_service(&e) => return Ok(()),
        Err(e) => return Err(format!("failed to open service: {e}")),
    };
    if let Ok(status) = service.query_status()
        && status.current_state != ServiceState::Stopped
    {
        let _ = service.stop();
    }
    service
        .delete()
        .map_err(|e| format!("failed to delete service: {e}"))
}

fn service_info() -> ServiceInfo {
    ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: std::env::current_exe().unwrap_or_default(),
        launch_arguments: vec![
            OsString::from(ARG_HOST),
            OsString::from(ARG_OWNER),
            OsString::from(crate::owner_arg().unwrap_or_else(current_user_sid)),
        ],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    }
}

fn start_if_stopped(service: &windows_service::service::Service) -> Result<(), String> {
    let running = service
        .query_status()
        .map(|s| s.current_state == ServiceState::Running)
        .unwrap_or(false);
    if running {
        return Ok(());
    }
    service
        .start(&[] as &[&std::ffi::OsStr])
        .map_err(|e| format!("failed to start service: {e}"))
}

fn open_manager(access: ServiceManagerAccess) -> Result<ServiceManager, String> {
    ServiceManager::local_computer(None::<&str>, access)
        .map_err(|e| format!("failed to open service manager: {e}"))
}

fn is_missing_service(e: &windows_service::Error) -> bool {
    matches!(e, windows_service::Error::Winapi(io)
        if io.raw_os_error() == Some(ERR_NO_SERVICE))
}

fn query_state() -> Result<Option<ServiceState>, String> {
    let manager = open_manager(ServiceManagerAccess::CONNECT)?;
    let service = match manager.open_service(SERVICE_NAME, ServiceAccess::QUERY_STATUS) {
        Ok(s) => s,
        Err(e) if is_missing_service(&e) => return Ok(None),
        Err(e) => return Err(format!("failed to open service: {e}")),
    };
    let status = service
        .query_status()
        .map_err(|e| format!("failed to query service status: {e}"))?;
    Ok(Some(status.current_state))
}

async fn wait_for_state(want_running: bool) -> Result<(), String> {
    let deadline = tokio::time::Instant::now() + STATE_TIMEOUT;
    loop {
        let running = matches!(query_state(), Ok(Some(ServiceState::Running)));
        if running == want_running {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "the service did not reach the {} state in time",
                if want_running { "running" } else { "stopped" }
            ));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn request(req: &Request, connect_grace: Duration) -> Result<Response, String> {
    let deadline = tokio::time::Instant::now() + connect_grace;
    let mut client = loop {
        match ClientOptions::new().open(PIPE_NAME) {
            Ok(c) => break c,
            // ERROR_FILE_NOT_FOUND / ERROR_PIPE_BUSY: the host is still coming up.
            Err(e)
                if tokio::time::Instant::now() < deadline
                    && matches!(e.raw_os_error(), Some(2) | Some(231)) =>
            {
                tokio::time::sleep(Duration::from_millis(150)).await;
            }
            Err(e) => return Err(format!("cannot reach the Nyx service: {e}")),
        }
    };

    let payload = serde_json::to_vec(req).map_err(|e| e.to_string())?;
    client
        .write_all(&payload)
        .await
        .map_err(|e| format!("failed to send request: {e}"))?;

    let mut buf = vec![0u8; 8192];
    let n = client
        .read(&mut buf)
        .await
        .map_err(|e| format!("failed to read response: {e}"))?;
    serde_json::from_slice(&buf[..n]).map_err(|e| format!("invalid service response: {e}"))
}

fn elevate(args: &[&str]) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let code = run_elevated(&exe, args, true)?;
    match code {
        Some(0) | None => Ok(()),
        // The helper runs hidden, so its own message only reaches the service log.
        Some(code) if code == HELPER_FAILURE as u32 => Err(format!(
            "the administrator helper failed — see the log in {}",
            logging::log_dir().display()
        )),
        Some(code) => Err(format!("the administrator helper exited with code {code}")),
    }
}

/// Starts `exe` through the UAC prompt; with `wait`, returns its exit code once it ends.
pub(crate) fn run_elevated(
    exe: &std::path::Path,
    args: &[&str],
    wait: bool,
) -> Result<Option<u32>, String> {
    use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED};
    use windows::Win32::System::Threading::{GetExitCodeProcess, INFINITE, WaitForSingleObject};
    use windows::Win32::UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
    use windows::core::PCWSTR;

    let wide = |s: &std::ffi::OsStr| {
        use std::os::windows::ffi::OsStrExt;
        s.encode_wide().chain(Some(0)).collect::<Vec<u16>>()
    };
    let verb = wide("runas".as_ref());
    let file = wide(exe.as_os_str());
    let params = wide(
        args.iter()
            .map(|a| quote_arg(a))
            .collect::<Vec<_>>()
            .join(" ")
            .as_ref(),
    );
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    if let Err(e) = unsafe { ShellExecuteExW(&mut info) } {
        return Err(if e.code() == ERROR_CANCELLED.to_hresult() {
            "the administrator prompt was cancelled".to_string()
        } else {
            format!("cannot request administrator rights: {e}")
        });
    }
    let process = info.hProcess;
    if process.is_invalid() {
        return Ok(None);
    }
    let mut code = None;
    if wait {
        let mut exit = 0u32;
        unsafe {
            WaitForSingleObject(process, INFINITE);
            if GetExitCodeProcess(process, &mut exit).is_ok() {
                code = Some(exit);
            }
        }
    }
    unsafe {
        let _ = CloseHandle(process);
    }
    Ok(code)
}

/// One argument quoted the way `CommandLineToArgvW` reads it back.
fn quote_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
        return arg.to_string();
    }
    let mut out = String::from('"');
    let mut slashes = 0;
    for c in arg.chars() {
        match c {
            '\\' => slashes += 1,
            '"' => {
                out.push_str(&"\\".repeat(slashes * 2 + 1));
                out.push('"');
                slashes = 0;
            }
            _ => {
                out.push_str(&"\\".repeat(slashes));
                out.push(c);
                slashes = 0;
            }
        }
    }
    out.push_str(&"\\".repeat(slashes * 2));
    out.push('"');
    out
}

/// Stops the service and waits for it, so its exe can be overwritten; fine if it is missing.
pub(crate) fn stop_for_update() -> bool {
    let Ok(manager) = open_manager(ServiceManagerAccess::CONNECT) else {
        return false;
    };
    let Ok(service) = manager.open_service(
        SERVICE_NAME,
        ServiceAccess::STOP | ServiceAccess::QUERY_STATUS,
    ) else {
        return false;
    };
    let running = service
        .query_status()
        .map(|s| s.current_state != ServiceState::Stopped)
        .unwrap_or(false);
    if running {
        let _ = service.stop();
        wait_for_stopped_blocking(&service);
    }
    running
}

/// SID of the current user, used to lock the IPC pipe to them.
fn current_user_sid() -> String {
    unsafe { current_user_sid_raw() }.unwrap_or_else(|| "IU".to_string())
}

unsafe fn current_user_sid_raw() -> Option<String> {
    use std::ptr;

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn OpenProcessToken(
            process: *mut std::ffi::c_void,
            desired_access: u32,
            token_handle: *mut *mut std::ffi::c_void,
        ) -> i32;
        fn GetTokenInformation(
            token_handle: *mut std::ffi::c_void,
            token_information_class: u32,
            token_information: *mut std::ffi::c_void,
            token_information_length: u32,
            return_length: *mut u32,
        ) -> i32;
        fn ConvertSidToStringSidW(sid: *mut std::ffi::c_void, string_sid: *mut *mut u16) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        fn LocalFree(mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    }

    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_USER_CLASS: u32 = 1;

    unsafe {
        let mut token: *mut std::ffi::c_void = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return None;
        }

        let mut size: u32 = 0;
        GetTokenInformation(token, TOKEN_USER_CLASS, ptr::null_mut(), 0, &mut size);
        let mut buffer = vec![0u8; size as usize];
        let ok = GetTokenInformation(
            token,
            TOKEN_USER_CLASS,
            buffer.as_mut_ptr() as *mut std::ffi::c_void,
            size,
            &mut size,
        );
        CloseHandle(token);
        if ok == 0 || buffer.len() < std::mem::size_of::<*mut std::ffi::c_void>() {
            return None;
        }

        let sid = *(buffer.as_ptr() as *const *mut std::ffi::c_void);
        let mut raw: *mut u16 = ptr::null_mut();
        if ConvertSidToStringSidW(sid, &mut raw) == 0 || raw.is_null() {
            return None;
        }
        let len = (0..).take_while(|&i| *raw.add(i) != 0).count();
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(raw, len));
        LocalFree(raw as *mut std::ffi::c_void);
        Some(text)
    }
}
