use std::ffi::OsString;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::ClientOptions;
use windows_service::service::{
    ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceState, ServiceType,
};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};

use crate::control::Status;
use crate::host::windows::PIPE_NAME;
use crate::protocol::{CoreSpec, PROTOCOL_VERSION, Request, Response};
use crate::{
    ARG_HOST, ARG_INSTALL, ARG_OWNER, ARG_UNINSTALL, SERVICE_DISPLAY_NAME, SERVICE_NAME,
    is_elevated,
};

const ERR_NO_SERVICE: i32 = 1060;
const CONNECT_GRACE: Duration = Duration::from_secs(10);
const STATE_TIMEOUT: Duration = Duration::from_secs(15);

pub async fn status() -> Result<Status, String> {
    let Some(state) = query_state()? else {
        return Ok(Status::NotInstalled);
    };
    if state != ServiceState::Running {
        return Ok(Status::Stopped);
    }
    match ping().await {
        Ok(_) => Ok(Status::Running),
        Err(reason) => Ok(Status::Stale { reason }),
    }
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

pub async fn start_core(spec: &CoreSpec) -> Result<u32, String> {
    match status().await? {
        Status::NotInstalled => return Err("the Nyx service is not installed".into()),
        Status::Running => {}
        _ => install().await?,
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
    match request(&Request::Ping, Duration::ZERO).await? {
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

    match manager.open_service(
        SERVICE_NAME,
        ServiceAccess::CHANGE_CONFIG | ServiceAccess::START | ServiceAccess::QUERY_STATUS,
    ) {
        Ok(service) => {
            service
                .change_config(&info)
                .map_err(|e| format!("failed to reconfigure service: {e}"))?;
            start_if_stopped(&service)
        }
        Err(e) if is_missing_service(&e) => {
            let service = manager
                .create_service(&info, ServiceAccess::START | ServiceAccess::QUERY_STATUS)
                .map_err(|e| format!("failed to create service: {e}"))?;
            let _ = service.set_description("Runs the mihomo core for Nyx");
            start_if_stopped(&service)
        }
        Err(e) => Err(format!("failed to open service: {e}")),
    }
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
    let arg_list = args
        .iter()
        .map(|a| format!("'{}'", a.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(",");
    let script = format!(
        "$p = Start-Process -FilePath '{}' -ArgumentList {} -Verb RunAs -WindowStyle Hidden -PassThru -Wait; exit $p.ExitCode",
        exe.display().to_string().replace('\'', "''"),
        arg_list
    );

    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("failed to request elevation: {e}"))?;

    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(if stderr.is_empty() {
        "the elevation prompt was cancelled".to_string()
    } else {
        stderr
    })
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
