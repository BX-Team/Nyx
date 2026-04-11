#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Some(code) = nyx_lib::service_host::maybe_run_as_service_from_args() {
        std::process::exit(code);
    }
    nyx_lib::run()
}
