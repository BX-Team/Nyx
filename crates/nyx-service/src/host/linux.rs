use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

use crate::host::{CoreManager, decode, encode};
use crate::logging;
use crate::protocol::{Request, Response};

pub const SOCKET_PATH: &str = "/run/nyx/nyx.sock";

const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub fn run_host(owner_uid: u32) -> i32 {
    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            logging::log(&format!("failed to build tokio runtime: {e}"));
            return 1;
        }
    };
    rt.block_on(async move {
        match serve(owner_uid).await {
            Ok(()) => 0,
            Err(e) => {
                logging::log(&format!("host error: {e}"));
                1
            }
        }
    })
}

async fn serve(owner_uid: u32) -> Result<(), String> {
    let path = Path::new(SOCKET_PATH);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("cannot create {parent:?}: {e}"))?;
    }
    // A leftover socket from a hard kill would make bind fail with EADDRINUSE.
    let _ = std::fs::remove_file(path);

    let listener =
        UnixListener::bind(path).map_err(|e| format!("cannot bind {SOCKET_PATH}: {e}"))?;
    restrict_socket(path, owner_uid);
    logging::log(&format!(
        "host listening on {SOCKET_PATH} for uid {owner_uid}"
    ));

    let mut manager = CoreManager::default();
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|e| format!("cannot install SIGTERM handler: {e}"))?;
    let mut int = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
        .map_err(|e| format!("cannot install SIGINT handler: {e}"))?;

    loop {
        tokio::select! {
            _ = term.recv() => break,
            _ = int.recv() => break,
            accepted = listener.accept() => {
                let Ok((mut stream, _)) = accepted else { continue };
                match peer_uid(&stream) {
                    Some(uid) if uid == owner_uid || uid == 0 => {}
                    Some(uid) => {
                        logging::log(&format!("rejected connection from uid {uid}"));
                        continue;
                    }
                    None => {
                        logging::log("rejected connection with unknown peer credentials");
                        continue;
                    }
                }
                let _ = tokio::time::timeout(
                    CLIENT_TIMEOUT,
                    serve_connection(&mut stream, &mut manager),
                )
                .await;
            }
        }
    }

    logging::log("host stopping");
    manager.handle(Request::StopCore).await;
    let _ = std::fs::remove_file(path);
    Ok(())
}

async fn serve_connection(stream: &mut UnixStream, manager: &mut CoreManager) {
    let mut buf = vec![0u8; 8192];
    let n = match stream.read(&mut buf).await {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let response = match decode(&buf[..n]) {
        Ok(req) => manager.handle(req).await,
        Err(message) => Response::Error { message },
    };
    let _ = stream.write_all(&encode(&response)).await;
    let _ = stream.shutdown().await;
}

fn restrict_socket(path: &Path, owner_uid: u32) {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c_path) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
        return;
    };
    if unsafe { libc::chmod(c_path.as_ptr(), 0o600) } != 0 {
        logging::log(&format!(
            "cannot chmod {SOCKET_PATH}: {}",
            std::io::Error::last_os_error()
        ));
    }
    // gid -1 leaves the group alone.
    if unsafe { libc::chown(c_path.as_ptr(), owner_uid, u32::MAX) } == 0 {
        return;
    }
    logging::log(&format!(
        "cannot chown {SOCKET_PATH} to uid {owner_uid} ({}); falling back to peer-credential \
         checks only",
        std::io::Error::last_os_error()
    ));
    if unsafe { libc::chmod(c_path.as_ptr(), 0o666) } != 0 {
        logging::log(&format!(
            "cannot chmod {SOCKET_PATH}: {}",
            std::io::Error::last_os_error()
        ));
    }
}

fn peer_uid(stream: &UnixStream) -> Option<u32> {
    let mut cred = libc::ucred {
        pid: 0,
        uid: u32::MAX,
        gid: u32::MAX,
    };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    let ok = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            &mut cred as *mut libc::ucred as *mut libc::c_void,
            &mut len,
        )
    };
    (ok == 0).then_some(cred.uid)
}
