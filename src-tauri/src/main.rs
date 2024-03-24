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
mod window;

use application::{create_application, use_application};
use command::CommandProvider;
use menu::{ApplicationMenu, MenuProvider};
use tauri::RunEvent;

fn main() {
    let app = tauri::Builder::default()
        .menu(ApplicationMenu::create_menu())
        .on_menu_event(|event| ApplicationMenu::handle_menu(event).unwrap())
        .setup(|app| {
            let application = create_application(app)?;
            application.init()?;
            application.run()?;
            Ok(())
        })
        .attach_commands()
        .build(tauri::generate_context!())
        .expect("error while running application");

    app.run(|_, event| match event {
        RunEvent::Exit => {
            use_application().drop_storage().ok();
        }
        _ => {}
    });
}
