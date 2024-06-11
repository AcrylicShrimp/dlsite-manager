mod fetch_new_products;
mod refresh_products_all;
mod scan_downloaded_products;

use self::{
    fetch_new_products::fetch_new_products, refresh_products_all::refresh_products_all,
    scan_downloaded_products::scan_downloaded_products,
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
                        "product/fetch-new-products",
                        "Fetch New Products",
                    ))
                    .add_item(CustomMenuItem::new(
                        "product/scan-downloaded-products",
                        "Scan Downloaded Products",
                    ))
                    .add_native_item(MenuItem::Separator)
                    .add_item(CustomMenuItem::new(
                        "product/refresh-products-all",
                        "Refresh All Products (Drop Caches)",
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
            "product/fetch-new-products" => {
                spawn((|| async {
                    {
                        let mut is_updating_product = use_application().is_updating_product();

                        if *is_updating_product {
                            return ();
                        }

                        *is_updating_product = true;
                    }

                    let result = fetch_new_products().await;
                    *use_application().is_updating_product() = false;

                    result.unwrap();
                })());
            }
            "product/refresh-products-all" => {
                spawn((|| async {
                    {
                        let mut is_updating_product = use_application().is_updating_product();

                        if *is_updating_product {
                            return ();
                        }

                        *is_updating_product = true;
                    }

                    let result = refresh_products_all().await;
                    *use_application().is_updating_product() = false;

                    result.unwrap();
                })());
            }
            "product/scan-downloaded-products" => {
                spawn((|| async {
                    {
                        let mut is_updating_product = use_application().is_updating_product();

                        if *is_updating_product {
                            return ();
                        }

                        *is_updating_product = true;
                    }

                    let result = scan_downloaded_products().await;
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
