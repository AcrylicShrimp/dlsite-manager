use crate::{
    application::use_application,
    application_error::Result,
    window::{AccountManagementWindow, BuildableWindow},
};
use tauri::{CustomMenuItem, Menu, MenuItem, Submenu, WindowMenuEvent};

pub struct ApplicationMenu;

pub trait MenuProvider {
    fn create_menu() -> Menu;
    fn handle_menu(event: WindowMenuEvent) -> Result<()>;
}

impl MenuProvider for ApplicationMenu {
    fn create_menu() -> Menu {
        Menu::new()
            .add_native_item(MenuItem::Quit)
            .add_submenu(Submenu::new(
                "Account",
                Menu::new().add_item(CustomMenuItem::new(
                    "account/open-account-management",
                    "Open Account Management",
                )),
            ))
    }

    fn handle_menu(event: WindowMenuEvent) -> Result<()> {
        let application = use_application();

        match event.menu_item_id() {
            "account/open-account-management" => {
                AccountManagementWindow.build_or_focus(application.app_handle())?;
            }
            _ => {}
        }

        Ok(())
    }
}
