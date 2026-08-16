use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};

/// Fixed loopback port for the single-instance lock / deep-link IPC.
const PORT: u16 = 47654;

/// Arg telling the relaunched process to wait for the dying instance to free the port.
pub const RELAUNCH_FLAG: &str = "--nyx-relaunch";

/// Returns the bound listener for the primary instance, or `None` when another
/// instance owns it (the deep link is already forwarded, so the caller exits).
pub fn acquire_or_forward() -> Option<TcpListener> {
    let relaunch = std::env::args().any(|a| a == RELAUNCH_FLAG);
    // A relaunch (restart) races the dying instance for the port; wait it out.
    let deadline = Instant::now()
        + if relaunch {
            Duration::from_secs(6)
        } else {
            Duration::ZERO
        };
    loop {
        match TcpListener::bind(("127.0.0.1", PORT)) {
            Ok(listener) => return Some(listener),
            Err(_) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(150));
            }
            Err(_) => {
                forward_deep_link();
                return None;
            }
        }
    }
}

/// Forwards our `nyx://` argument, or — with no deep link — asks the primary
/// instance to show its window, which on Linux may need recreating.
fn forward_deep_link() {
    let url = deep_link_arg().unwrap_or_else(|| "nyx://show".to_string());
    if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", PORT)) {
        let _ = stream.write_all(url.as_bytes());
    }
}

pub fn deep_link_arg() -> Option<String> {
    std::env::args().find(|a| a.starts_with("nyx://"))
}

pub fn serve(listener: TcpListener, tx: std::sync::mpsc::Sender<String>) {
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                continue;
            };
            let mut buf = String::new();
            if stream.read_to_string(&mut buf).is_ok() {
                let url = buf.trim().to_string();
                if !url.is_empty() {
                    let _ = tx.send(url);
                }
            }
        }
    });
}
