use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::control::windows::{run_elevated, stop_for_update};
use crate::{ARG_UPDATE, logging};

/// How long the app gets to quit by itself before the helper ends it.
const APP_EXIT_WAIT: Duration = Duration::from_secs(15);
const COPY_ATTEMPTS: u32 = 30;

/// Hands `new_exe` to an elevated copy of this build, which swaps it in once the app has quit.
pub fn apply_update(new_exe: &Path) -> Result<(), String> {
    let installed = std::env::current_exe().map_err(|e| e.to_string())?;
    // The installed exe is locked while it runs, so the helper runs from a copy.
    let helper = new_exe.with_file_name("nyx-update-helper.exe");
    std::fs::copy(&installed, &helper)
        .map_err(|e| format!("cannot prepare the update helper: {e}"))?;
    let pid = std::process::id().to_string();
    run_elevated(
        &helper,
        &[
            ARG_UPDATE,
            &new_exe.display().to_string(),
            &installed.display().to_string(),
            &pid,
        ],
        false,
    )
    .map(|_| ())
}

/// The elevated side: wait for the app, stop the service, copy the new exe, bring both back.
pub fn apply_here(args: &[String]) -> Result<(), String> {
    let at = args
        .iter()
        .position(|a| a == ARG_UPDATE)
        .ok_or("missing update arguments")?;
    let [new_exe, installed, pid] =
        [1, 2, 3].map(|i| args.get(at + i).cloned().unwrap_or_default());
    let (new_exe, installed) = (PathBuf::from(new_exe), PathBuf::from(installed));
    if !new_exe.is_file() || installed.as_os_str().is_empty() {
        return Err("bad update arguments".into());
    }
    logging::log(&format!(
        "updating {} from {}",
        installed.display(),
        new_exe.display()
    ));

    if let Ok(pid) = pid.parse::<u32>() {
        wait_for_exit(pid);
    }
    let service_was_running = stop_for_update();

    let mut copied = Err(String::new());
    for _ in 0..COPY_ATTEMPTS {
        copied = std::fs::copy(&new_exe, &installed)
            .map(|_| ())
            .map_err(|e| e.to_string());
        if copied.is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    if service_was_running {
        let _ = crate::control::windows::control_here("start");
    }
    // Explorer starts the app as the signed-in user rather than as this elevated helper.
    let _ = std::process::Command::new("explorer.exe")
        .arg(&installed)
        .spawn();
    copied.map_err(|e| format!("cannot replace {}: {e}", installed.display()))
}

fn wait_for_exit(pid: u32) {
    use windows::Win32::Foundation::{CloseHandle, WAIT_TIMEOUT};
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, TerminateProcess, WaitForSingleObject,
    };
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, false, pid) else {
            return;
        };
        if WaitForSingleObject(process, APP_EXIT_WAIT.as_millis() as u32) == WAIT_TIMEOUT {
            logging::log("the app did not quit in time, ending it");
            let _ = TerminateProcess(process, 1);
            WaitForSingleObject(process, 5000);
        }
        let _ = CloseHandle(process);
    }
}
