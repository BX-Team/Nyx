use std::io::Write;
use std::path::{Path, PathBuf};

/// A system service cannot reach the GUI user's data dir, so the host logs here.
pub fn log_dir() -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from("C:\\ProgramData\\Nyx")
    }
    #[cfg(not(windows))]
    {
        PathBuf::from("/var/log/nyx")
    }
}

pub fn log(msg: &str) {
    let dir = log_dir();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(format!("{today}.log")))
    {
        let ts = chrono::Local::now().format("%H:%M:%S%.3f");
        let _ = writeln!(f, "[{ts}] {msg}");
    }
}

pub fn clean_old(max_days: u32) {
    if max_days == 0 {
        return;
    }
    let Some(cutoff) = chrono::Local::now()
        .date_naive()
        .checked_sub_signed(chrono::Duration::days(max_days as i64 - 1))
    else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(log_dir()) else {
        return;
    };
    for path in entries.filter_map(|e| e.ok()).map(|e| e.path()) {
        if is_expired_log(&path, cutoff) {
            let _ = std::fs::remove_file(&path);
        }
    }
}

fn is_expired_log(path: &Path, cutoff: chrono::NaiveDate) -> bool {
    if path.extension().and_then(|s| s.to_str()) != Some("log") {
        return false;
    }
    path.file_stem()
        .and_then(|s| s.to_str())
        .and_then(|stem| chrono::NaiveDate::parse_from_str(stem, "%Y-%m-%d").ok())
        .map(|date| date < cutoff)
        .unwrap_or(false)
}
