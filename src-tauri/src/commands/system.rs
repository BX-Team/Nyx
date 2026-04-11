use serde_json::Value;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_shell::ShellExt;

#[tauri::command]
pub async fn check_auto_run(app: AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn enable_auto_run(app: AppHandle) -> Result<(), String> {
    app.autolaunch().enable().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn disable_auto_run(app: AppHandle) -> Result<(), String> {
    app.autolaunch().disable().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_interfaces() -> Result<Vec<Value>, String> {
    let ifaces = if_addrs::get_if_addrs().map_err(|e| e.to_string())?;
    let result: Vec<Value> = ifaces
        .into_iter()
        .map(|iface| {
            serde_json::json!({
                "name": iface.name,
                "addr": iface.ip().to_string(),
                "is_loopback": iface.is_loopback(),
            })
        })
        .collect();
    Ok(result)
}

#[tauri::command]
pub async fn open_uwp_tool(app: AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        app.shell()
            .sidecar("enableLoopback")
            .map_err(|e| e.to_string())?
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(windows))]
    let _ = app;
    Ok(())
}

#[tauri::command]
pub async fn setup_firewall() -> Result<(), String> {
    #[cfg(windows)]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        std::process::Command::new("netsh")
            .args([
                "advfirewall", "firewall", "add", "rule",
                "name=Nyx", "dir=in", "action=allow",
                &format!("program={}", exe.display()),
            ])
            .output()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn find_system_mihomo() -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let names = ["mihomo", "clash"];

    for name in names {
        #[cfg(windows)]
        let cmd = "where";
        #[cfg(not(windows))]
        let cmd = "which";

        if let Ok(out) = std::process::Command::new(cmd).arg(name).output() {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    let p = line.trim().to_string();
                    if !p.is_empty() && std::path::Path::new(&p).exists() && !found.contains(&p) {
                        found.push(p);
                    }
                }
            }
        }
    }

    #[cfg(not(windows))]
    {
        let common_dirs = ["/usr/bin", "/usr/local/bin", "/opt/homebrew/bin"];
        for dir in common_dirs {
            for name in names {
                let path = format!("{}/{}", dir, name);
                if std::path::Path::new(&path).exists() && !found.contains(&path) {
                    found.push(path);
                }
            }
        }
    }

    found.sort();
    found.dedup();
    found
}

#[cfg(windows)]
const ELEVATE_TASK_NAME: &str = "NyxElevated";

#[tauri::command]
pub async fn check_elevate_task() -> Result<bool, String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let out = std::process::Command::new("schtasks")
            .args(["/query", "/tn", ELEVATE_TASK_NAME])
            .creation_flags(0x08000000) 
            .output()
            .map_err(|e| e.to_string())?;
        return Ok(out.status.success());
    }
    #[cfg(not(windows))]
    Ok(false)
}

#[tauri::command]
pub async fn create_elevate_task() -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let exe_str = exe.to_string_lossy().to_string();

        let tmp_dir = std::env::temp_dir();
        let script_path = tmp_dir.join("nyx_create_task.ps1");
        let script = format!(
            r#"$action = New-ScheduledTaskAction -Execute '"{exe}"'
$trigger = New-ScheduledTaskTrigger -AtLogOn
$principal = New-ScheduledTaskPrincipal -RunLevel Highest -UserId $env:USERNAME
$settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit 0 -MultipleInstances IgnoreNew
Register-ScheduledTask -TaskName '{task}' -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Force | Out-Null"#,
            exe = exe_str.replace('\'', "''"),
            task = ELEVATE_TASK_NAME,
        );
        std::fs::write(&script_path, &script).map_err(|e| e.to_string())?;

        let launcher = format!(
            "Start-Process powershell -ArgumentList '-NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File \"{script}\"' -Verb RunAs -Wait",
            script = script_path.to_string_lossy().replace('"', "`\"")
        );

        let out = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &launcher])
            .creation_flags(0x08000000) 
            .output()
            .map_err(|e| e.to_string())?;

        let _ = std::fs::remove_file(&script_path);

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            } else {
                stderr
            });
        }
        return Ok(());
    }
    #[cfg(not(windows))]
    Ok(())
}

#[tauri::command]
pub async fn delete_elevate_task() -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let out = std::process::Command::new("schtasks")
            .args(["/delete", "/tn", ELEVATE_TASK_NAME, "/f"])
            .creation_flags(0x08000000) 
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout).to_string()
                + &String::from_utf8_lossy(&out.stderr);
            if !text.contains("cannot find") && !text.contains("does not exist") && !text.contains("1060") {
                return Err(text.trim().to_string());
            }
        }
        return Ok(());
    }
    #[cfg(not(windows))]
    Ok(())
}
