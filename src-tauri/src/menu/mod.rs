mod refresh_product_download;
mod refresh_product_list;
mod update_product_list;

use self::{
    refresh_product_download::refresh_product_download, refresh_product_list::refresh_product_list,
    update_product_list::update_product_list,
};
use crate::{
    application::use_application,
    application_error::Result,
    window::{AccountManagementWindow, BuildableWindow, SettingWindow},
};
use tauri::{async_runtime::spawn, CustomMenuItem, Menu, MenuItem, Submenu, WindowMenuEvent};

pub struct ApplicationMenu;

pub trait MenuProvider {
    fn create_menu() -> Menu;
    fn handle_menu(event: WindowMenuEvent) -> Result<()>;
}

impl MenuProvider for ApplicationMenu {
    fn create_menu() -> Menu {
        Menu::new()
            .add_submenu(Submenu::new(
                "Window",
                Menu::new()
                    .add_native_item(MenuItem::EnterFullScreen)
                    .add_native_item(MenuItem::Minimize)
                    .add_native_item(MenuItem::CloseWindow)
                    .add_native_item(MenuItem::Quit),
            ))
            .add_submenu(Submenu::new(
                "Edit",
                Menu::new()
                    .add_native_item(MenuItem::Undo)
                    .add_native_item(MenuItem::Redo)
                    .add_native_item(MenuItem::Cut)
                    .add_native_item(MenuItem::Copy)
                    .add_native_item(MenuItem::Paste)
                    .add_native_item(MenuItem::SelectAll),
            ))
            .add_submenu(Submenu::new(
                "Account",
                Menu::new().add_item(CustomMenuItem::new(
                    "account/open-account-management",
                    "Open Account Management",
                )),
            ))
            .add_submenu(Submenu::new(
                "Product",
                Menu::new()
                    .add_item(CustomMenuItem::new(
                        "product/update-product-list",
                        "Update Product",
                    ))
                    .add_native_item(MenuItem::Separator)
                    .add_item(CustomMenuItem::new(
                        "product/refresh-product-list",
                        "Refresh Product All",
                    ))
                    .add_item(CustomMenuItem::new(
                        "product/refresh-product-download",
                        "Refresh Product Downloads",
                    )),
            ))
            .add_submenu(Submenu::new(
                "Setting",
                Menu::new().add_item(CustomMenuItem::new("setting/open-setting", "Open Settings")),
            ))
    }

    fn handle_menu(event: WindowMenuEvent) -> Result<()> {
        match event.menu_item_id() {
            "account/open-account-management" => {
                AccountManagementWindow.build_or_focus(use_application().app_handle())?;
            }
            "product/update-product-list" => {
                spawn((|| async {
                    {
                        let mut is_updating_product = use_application().is_updating_product();

                        if *is_updating_product {
                            return ();
                        }

                        *is_updating_product = true;
                    }

                    let result = update_product_list().await;
                    *use_application().is_updating_product() = false;

                    result.unwrap();
                })());
            }
            "product/refresh-product-list" => {
                spawn((|| async {
                    {
                        let mut is_updating_product = use_application().is_updating_product();

                        if *is_updating_product {
                            return ();
                        }

                        *is_updating_product = true;
                    }

                    let result = refresh_product_list().await;
                    *use_application().is_updating_product() = false;

                    result.unwrap();
                })());
            }
            "product/refresh-product-download" => {
                spawn((|| async {
                    {
                        let mut is_updating_product = use_application().is_updating_product();

                        if *is_updating_product {
                            return ();
                        }

                        *is_updating_product = true;
                    }

                    let result = refresh_product_list().await;

                    if result.is_err() {
                        *use_application().is_updating_product() = false;
                        return result.unwrap();
                    }

                    let result = refresh_product_download().await;
                    *use_application().is_updating_product() = false;

                    result.unwrap();
                })());
            }
            "setting/open-setting" => {
                SettingWindow.build_or_focus(use_application().app_handle())?;
            }
            _ => {}
        }

        Ok(())
    }
}
