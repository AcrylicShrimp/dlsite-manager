#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod application;
mod application_error;
mod command;
mod database;
mod dlsite;
mod menu;
mod services;
mod window;

use application::{create_application, use_application};
use command::CommandProvider;
use flexi_logger::{Cleanup, Criterion, FileSpec, Naming};
use menu::{create_menu, handle_menu};
use tauri::{Manager, RunEvent};

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .menu(|app| create_menu(app))
        .setup(|app| {
            app.on_menu_event(|_, event| {
                handle_menu(event).unwrap();
            });

            let application = create_application(app)?;
            application.init()?;
            application.run()?;
            Ok(())
        })
        .attach_commands()
        .build(tauri::generate_context!())
        .expect("error while running application");

    if let Ok(dir) = app.path().app_log_dir() {
        flexi_logger::Logger::try_with_str("info")
            .unwrap()
            .log_to_file(
                FileSpec::default()
                    .directory(dir)
                    .basename("dlsite-manager")
                    .suffix("txt"),
            )
            .rotate(
                Criterion::Size(10_000_000),
                Naming::Timestamps,
                Cleanup::KeepLogFiles(5),
            )
            .start()
            .expect("failed to init logger");
    }

    app.run(|_, event| match event {
        RunEvent::Exit => {
            use_application().drop_storage().ok();
        }
        _ => {}
    });
}
