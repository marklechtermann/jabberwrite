mod app;
mod audio;
mod config;
mod history;
mod hotkey;
mod keys;
mod overlay;
mod paste;
mod permissions;
mod stt;
mod update;

use std::sync::Arc;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};

use app::{App, Paths};
use keys::Shortcut;

pub const VERSION: &str = env!("JABBERWRITE_VERSION");
pub const REPO_URL: &str = "https://github.com/marklechtermann/jabberwrite";

fn paths() -> Paths {
    let base = directories::BaseDirs::new().expect("no home directory");
    let data = base.data_local_dir().join("jabberwrite");
    Paths {
        config: base.config_dir().join("jabberwrite").join("config.json"),
        logs: data.join("logs"),
        models: data.join("models"),
        history: data.join("history.jsonl"),
    }
}

fn init_logging(dir: &std::path::Path) {
    use tracing_subscriber::EnvFilter;
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("jabberwrite")
        .filename_suffix("log")
        .max_log_files(7)
        .build(dir);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false);
    match appender {
        Ok(appender) => builder.with_writer(appender).init(),
        Err(e) => {
            builder.init();
            tracing::error!("cannot open log directory {}: {e}", dir.display());
        }
    }
}

pub fn run() {
    let paths = paths();
    init_logging(&paths.logs);
    tracing::info!("jabberwrite {VERSION} starting");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|handle, _, _| {
            handle.state::<Arc<App>>().show_settings();
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            app::app_info,
            app::save_shortcut,
            app::set_hotkey_paused,
            app::open_path,
            app::history,
            app::clear_history,
            app::copy_text,
            app::save_clear_history_on_exit,
            app::open_url,
            app::permissions,
            app::open_permission_settings,
            update::check_update,
            update::install_update,
        ])
        .setup(move |tauri_app| {
            #[cfg(target_os = "macos")]
            tauri_app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let loaded = config::load(&paths.config);
            if let Some(e) = &loaded.error {
                tracing::error!("config: {e}");
            }
            let shortcut = Shortcut::parse(&loaded.config.shortcut).unwrap_or_else(|e| {
                tracing::error!("config: {e}; using default shortcut");
                Shortcut::parse(&config::Config::default().shortcut).unwrap()
            });
            let show_settings =
                loaded.error.is_some() || permissions::check().is_some_and(|p| !p.all_granted());

            let handle = tauri_app.handle().clone();
            let app = Arc::new(App::new(handle.clone(), paths, loaded.config, loaded.error));
            tauri_app.manage(app.clone());

            let status = MenuItem::with_id(&handle, "status", "Jabberwrite", false, None::<&str>)?;
            let menu = Menu::with_items(
                &handle,
                &[
                    &status,
                    &PredefinedMenuItem::separator(&handle)?,
                    &MenuItem::with_id(&handle, "settings", "Settings…", true, None::<&str>)?,
                    &MenuItem::with_id(
                        &handle,
                        "update",
                        "Check for Updates…",
                        true,
                        None::<&str>,
                    )?,
                    &MenuItem::with_id(&handle, "quit", "Quit", true, None::<&str>)?,
                ],
            )?;
            let tray = TrayIconBuilder::with_id("main")
                .icon(tauri_app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|handle, event| match event.id().as_ref() {
                    "settings" => handle.state::<Arc<App>>().show_settings(),
                    "update" => {
                        let app = handle.state::<Arc<App>>().inner().clone();
                        tauri::async_runtime::spawn(update::check_with_dialogs(app));
                    }
                    "quit" => handle.exit(0),
                    _ => {}
                })
                .build(tauri_app)?;
            app.set_tray(tray, status);

            overlay::create(&handle)?;

            let hotkey_app = app.clone();
            if let Err(e) = hotkey::start(shortcut, move || {
                let app = hotkey_app.clone();
                std::thread::spawn(move || app.toggle());
            }) {
                tracing::error!("{e}");
                app.set_status(&format!("Error: {e}"));
            }

            let loader = app.clone();
            std::thread::spawn(move || loader.load_model());

            if show_settings {
                app.show_settings();
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "settings" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building jabberwrite")
        .run(|handle, event| {
            if let tauri::RunEvent::Exit = event {
                handle.state::<Arc<App>>().clear_history_if_configured();
            }
        });
}
