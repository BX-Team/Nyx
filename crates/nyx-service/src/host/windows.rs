use std::ffi::{OsString, c_void};
use std::sync::mpsc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows::core::{BOOL, PCWSTR};
use windows_service::define_windows_service;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;

use crate::host::{CoreManager, decode, encode};
use crate::protocol::{Request, Response};
use crate::{ARG_OWNER, SERVICE_NAME, logging};

pub const PIPE_NAME: &str = r"\\.\pipe\nyx_mihomo_ipc";

const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

define_windows_service!(ffi_service_main, service_main);

pub fn run_dispatcher() -> i32 {
    match service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
        Ok(()) => 0,
        Err(e) => {
            logging::log(&format!("service dispatcher failed: {e}"));
            1
        }
    }
}

fn service_main(_arguments: Vec<OsString>) {
    if let Err(e) = run_service() {
        logging::log(&format!("service runtime error: {e}"));
    }
}

fn run_service() -> windows_service::Result<()> {
    logging::log("host starting");
    let (stop_tx, stop_rx) = mpsc::channel::<()>();

    let status_handle =
        service_control_handler::register(SERVICE_NAME, move |event| match event {
            ServiceControl::Stop => {
                let _ = stop_tx.send(());
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        })?;

    status_handle.set_service_status(status(ServiceState::Running, ServiceControlAccept::STOP))?;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    rt.block_on(async move {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        std::thread::spawn(move || {
            let _ = stop_rx.recv();
            let _ = shutdown_tx.send(());
        });
        serve(shutdown_rx).await;
    });

    status_handle
        .set_service_status(status(ServiceState::Stopped, ServiceControlAccept::empty()))?;
    logging::log("host stopped");
    Ok(())
}

fn status(state: ServiceState, accepted: ServiceControlAccept) -> ServiceStatus {
    ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: accepted,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(0),
        process_id: None,
    }
}

async fn serve(mut shutdown: tokio::sync::oneshot::Receiver<()>) {
    let mut manager = CoreManager::default();
    let mut security = pipe_security();

    let mut server = match create_pipe(security.as_mut(), true) {
        Ok(s) => s,
        Err(e) => {
            logging::log(&format!("cannot create pipe: {e}"));
            return;
        }
    };

    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            connected = server.connect() => {
                if let Err(e) = connected {
                    logging::log(&format!("pipe connect error: {e}"));
                    continue;
                }
                // Queue the next instance before serving, so a fast reconnect never misses it.
                let next = match create_pipe(security.as_mut(), false) {
                    Ok(s) => s,
                    Err(e) => {
                        logging::log(&format!("cannot create pipe instance: {e}"));
                        break;
                    }
                };
                let mut conn = std::mem::replace(&mut server, next);
                let _ = tokio::time::timeout(
                    CLIENT_TIMEOUT,
                    serve_connection(&mut conn, &mut manager),
                )
                .await;
                conn.disconnect().ok();
            }
        }
    }

    manager.handle(Request::StopCore).await;
}

async fn serve_connection(conn: &mut NamedPipeServer, manager: &mut CoreManager) {
    let mut buf = vec![0u8; 8192];
    let n = match conn.read(&mut buf).await {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let response = match decode(&buf[..n]) {
        Ok(req) => manager.handle(req).await,
        Err(message) => Response::Error { message },
    };
    let _ = conn.write_all(&encode(&response)).await;
    let _ = conn.flush().await;
}

fn create_pipe(
    security: Option<&mut SECURITY_ATTRIBUTES>,
    first: bool,
) -> std::io::Result<NamedPipeServer> {
    let mut opts = ServerOptions::new();
    opts.first_pipe_instance(first);
    match security {
        Some(sa) => unsafe {
            opts.create_with_security_attributes_raw(
                PIPE_NAME,
                sa as *mut SECURITY_ATTRIBUTES as *mut c_void,
            )
        },
        None => opts.create(PIPE_NAME),
    }
}

/// The pipe hands a SYSTEM process a binary path to run, so it is limited to
/// SYSTEM, administrators, and the user the service was installed for.
fn pipe_security() -> Option<SECURITY_ATTRIBUTES> {
    let owner = owner_sid_arg().unwrap_or_else(|| "IU".to_string());
    let sddl: Vec<u16> = format!("D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGWGX;;;{owner})\0")
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

fn owner_sid_arg() -> Option<String> {
    let mut args = std::env::args();
    while let Some(arg) = args.next() {
        if arg == ARG_OWNER {
            return args.next().filter(|s| !s.is_empty());
        }
    }
    None
}
