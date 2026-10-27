use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-env-changed=NYX_REV");

    let rev = std::env::var("NYX_REV").ok().or_else(|| {
        let out = Command::new("git")
            .args(["rev-parse", "--short=8", "HEAD"])
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
    });
    println!(
        "cargo:rustc-env=NYX_REV={}",
        rev.unwrap_or_else(|| "unknown".into())
    );
    println!(
        "cargo:rustc-env=NYX_GPUI_KIT={}",
        locked_version("gpui-kit")
    );

    #[cfg(windows)]
    windows_resources();
}

/// The exe's icon and the names Explorer, the task manager and antivirus scanners show.
#[cfg(windows)]
fn windows_resources() {
    println!("cargo:rerun-if-changed=assets/brand/icon.ico");
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/brand/icon.ico")
        .set("ProductName", "Nyx")
        .set("FileDescription", "Nyx — Mihomo proxy client")
        .set("CompanyName", "BX Team")
        .set("LegalCopyright", "BX Team, GPL-3.0-or-later")
        .set("OriginalFilename", "nyx.exe")
        .set("InternalName", "nyx");
    if let Err(e) = res.compile() {
        println!("cargo:warning=failed to embed Windows resources: {e}");
    }
}

/// The version Cargo.lock pins for `name`, so the About page names what was really built.
fn locked_version(name: &str) -> String {
    let lock = std::fs::read_to_string("Cargo.lock").unwrap_or_default();
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line == format!("name = \"{name}\"")
            && let Some(v) = lines.next().and_then(|l| l.strip_prefix("version = \""))
        {
            return v.trim_end_matches('"').to_owned();
        }
    }
    "?".into()
}
