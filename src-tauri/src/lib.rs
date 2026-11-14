//! MochaEduCardReporter app shell (Tauri v2): typed commands over the
//! `mocha-engine` crate, settings and history persistence, and the plugin
//! wiring. See `PLAN_REDO_CODE.md`, sections 3.5-3.7.
//!
//! Regenerate the TypeScript bindings (`src/lib/bindings.ts`) with
//! `UPDATE_BINDINGS=1 cargo test -p mocha-edu-card-reporter bindings`; a debug
//! run of the app also rewrites them. The same test fails when the committed
//! file is stale.

#![cfg_attr(test, allow(clippy::unwrap_used))]

pub mod commands;
pub mod error;
pub mod history;
pub mod paths;
pub mod persist;
pub mod services;
pub mod settings;
pub mod state;

use std::sync::Arc;

use tauri::Manager;
use tauri_specta::{collect_commands, Builder};

use crate::persist::TauriStore;
use crate::services::{Dirs, Services};

/// Where the generated bindings live, relative to this crate.
pub const BINDINGS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/lib/bindings.ts");

/// The command registry shared by the app and the bindings exporter.
pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        // One TypeScript type per Rust type (no `_Serialize`/`_Deserialize` split).
        .disable_serde_phases()
        .commands(collect_commands![
            commands::list_sheets,
            commands::inspect_gradebook,
            commands::preview_student,
            commands::generate_reports,
            commands::cancel_generation,
            commands::check_output_dir,
            commands::default_output_base,
            commands::read_pdf,
            commands::get_settings,
            commands::save_settings,
            commands::get_recent_files,
            commands::add_recent_file,
            commands::get_history,
            commands::add_history_entry,
            commands::remove_history_entry,
            commands::reveal_in_folder,
            commands::open_externally,
            commands::open_log_dir,
            commands::app_info,
        ])
}

/// Registers every plugin.
///
/// The updater needs its public key and endpoints in `tauri.conf.json`
/// (`plugins.updater`) and its capability in `capabilities/default.json`.
/// `tauri-plugin-process` provides the restart after installing an update.
fn with_plugins(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    let level = if cfg!(debug_assertions) {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    builder
        // Must come first: a second launch focuses the running window.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(level)
                .clear_targets()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("mocha-edu-card-reporter".into()),
                    },
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .max_file_size(1_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(5))
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
}

/// Starts the application.
///
/// # Panics
///
/// When Tauri cannot start (no window system, broken configuration): there is
/// nothing useful the app can do then.
pub fn run() {
    let builder = specta_builder();

    #[cfg(debug_assertions)]
    if let Err(e) = builder.export(specta_typescript::Typescript::default(), BINDINGS_PATH) {
        eprintln!("could not write the TypeScript bindings: {e}");
    }

    with_plugins(tauri::Builder::default())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app);
            let path = app.path();
            let dirs = Dirs {
                preview_dir: path.app_cache_dir()?.join("preview"),
                log_dir: path.app_log_dir()?,
                documents_dir: path.document_dir().ok(),
                home_dir: path.home_dir().ok(),
            };
            let store = TauriStore::open(app.handle())?;
            app.manage(Arc::new(Services::new(Arc::new(store), dirs)));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the MochaEduCardReporter application");
}

#[cfg(test)]
mod tests;
