#![allow(dead_code)]

rust_i18n::i18n!("locales", fallback = "en-US");

mod app;
mod backend;
mod ui;

pub fn run() {
    // On Windows, a copy of this exe is registered as the background service
    // host (`--nyx-service`). When launched that way, run the service dispatcher
    // and exit before touching any GUI.
    if let Some(code) = backend::maybe_run_as_service_from_args() {
        std::process::exit(code);
    }

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    log::info!("Nyx starting up (gpui)");

    // Single-instance guard: a second launch forwards its `nyx://` deep link to
    // the running instance and exits, so only one GUI is ever live.
    let Some(listener) = app::single_instance::acquire_or_forward() else {
        log::info!("another instance is running — forwarded deep link, exiting");
        return;
    };

    let app = gpui_platform::application().with_assets(app::assets::Assets);

    // Tidy the data dir before anything reads it (rename legacy config, drop the
    // stale Tauri window-state file).
    backend::startup::migrate_data_dir();
    let silent = backend::config::app_config_bool("silentStart");

    app.run(move |cx| {
        // Must run before using any gpui-component feature.
        gpui_component::init(cx);
        // Nyx is a dark-themed app; wire this to app config (appTheme) later.
        gpui_component::Theme::change(gpui_component::ThemeMode::Dark, None, cx);
        ui::theme::apply(cx);
        // Load persisted state and set the active locale.
        app::state::AppState::init(cx);
        if !silent {
            cx.activate(true);
        }
        ui::open_main_window(cx, silent);
        // Bring up the mihomo core + live data once the window exists.
        app::bootstrap::spawn_backend_startup(cx);
        // System tray (icon + menu) living in the gpui event loop.
        app::tray::init(cx);
        // Background scheduler: subscription auto-update + quota/expiry warnings.
        app::scheduler::init(cx);
        // Global hotkeys (re-registered from config once it loads).
        app::hotkeys::init(cx);
        // Deep links: register the `nyx://` scheme, then drain URLs forwarded by
        // later instances (plus our own launch arg) into the handler.
        app::deep_link::register_scheme();
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        if let Some(url) = app::single_instance::deep_link_arg() {
            let _ = tx.send(url);
        }
        app::single_instance::serve(listener, tx);
        app::deep_link::start(rx, cx);
    });
}
