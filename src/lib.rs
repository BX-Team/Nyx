#![allow(dead_code)]

rust_i18n::i18n!("locales", fallback = "en-US");

mod app;
mod backend;
mod ui;

pub fn run() {
    // Service host and the elevated install/uninstall helpers run without a GUI.
    if let Some(code) = nyx_service::maybe_run_service_mode() {
        std::process::exit(code);
    }

    // Graphics driver probing and the tray's D-Bus chatter log at info on healthy systems.
    env_logger::Builder::from_env(
        env_logger::Env::new().filter_or("NYX_LOG", "warn,wgpu_hal=off,zbus=error"),
    )
    .format_timestamp_millis()
    .init();
    log::info!("Nyx starting up (gpui)");

    // A second launch forwards its `nyx://` deep link to the running instance and exits.
    let Some(listener) = app::single_instance::acquire_or_forward() else {
        log::info!("another instance is running — forwarded deep link, exiting");
        return;
    };

    let app = gpui::application().with_assets(app::assets::Assets);

    backend::startup::migrate_data_dir();
    let silent = backend::config::app_config_bool("silentStart");

    app.run(move |cx| {
        gpui::init(cx);
        ui::theme::init(cx);
        ui::init_keys(cx);
        app::state::AppState::init(cx);
        ui::set_labels(cx);
        // Closing the window hides Nyx to the tray; the tray keeps the process alive.
        #[cfg(not(windows))]
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        if !silent {
            cx.activate(true);
        }
        ui::open_main_window(cx, silent);
        app::bootstrap::spawn_backend_startup(cx);
        app::tray::init(cx);
        app::scheduler::init(cx);
        app::hotkeys::init(cx);
        // Register the `nyx://` scheme, then drain URLs forwarded by other instances.
        app::deep_link::register_scheme();
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        if let Some(url) = app::single_instance::deep_link_arg() {
            let _ = tx.send(url);
        }
        app::single_instance::serve(listener, tx);
        app::deep_link::start(rx, cx);
    });
}
