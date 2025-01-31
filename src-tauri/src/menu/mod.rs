mod fetch_new_products;
mod refresh_products_all;
mod scan_downloaded_products;

use self::{
    fetch_new_products::fetch_new_products, refresh_products_all::refresh_products_all,
    scan_downloaded_products::scan_downloaded_products,
};
use crate::{
    application::use_application,
    window::{AccountManagementWindow, BuildableWindow, SettingWindow},
};
use anyhow::Error as AnyError;
use tauri::{
    async_runtime::spawn,
    menu::{Menu, MenuEvent, SubmenuBuilder},
    Manager, Runtime,
};
use tauri_plugin_opener::OpenerExt;

pub fn create_menu<R: Runtime>(manager: &impl Manager<R>) -> Result<Menu<R>, tauri::Error> {
    let menu = Menu::new(manager)?;

    menu.append(
        &SubmenuBuilder::new(manager, "Window")
            .fullscreen()
            .minimize()
            .maximize()
            .close_window()
            .separator()
            .quit()
            .build()?,
    )?;
    menu.append(
        &SubmenuBuilder::new(manager, "Edit")
            .undo()
            .redo()
            .cut()
            .copy()
            .paste()
            .select_all()
            .build()?,
    )?;
    menu.append(
        &SubmenuBuilder::new(manager, "Account")
            .text("account/open-account-management", "Open Account Management")
            .build()?,
    )?;
    menu.append(
        &SubmenuBuilder::new(manager, "Product")
            .text("product/fetch-new-products", "Fetch New Products")
            .text(
                "product/scan-downloaded-products",
                "Scan Downloaded Products",
            )
            .separator()
            .text(
                "product/refresh-products-all",
                "Refresh All Products (Drop Caches)",
            )
            .build()?,
    )?;
    menu.append(
        &SubmenuBuilder::new(manager, "Setting")
            .text("setting/open-setting", "Open Settings")
            .build()?,
    )?;
    menu.append(
        &SubmenuBuilder::new(manager, "Log")
            .text("log/open-log-directory", "Open Log Directory")
            .build()?,
    )?;

    Ok(menu)
}

pub fn handle_menu(event: MenuEvent) -> Result<(), AnyError> {
    match event.id.as_ref() {
        "account/open-account-management" => {
            AccountManagementWindow.build_or_focus(use_application().app_handle())?;
        }
        "product/fetch-new-products" => {
            spawn(async {
                {
                    let mut is_updating_product = use_application().is_updating_product();

                    if *is_updating_product {
                        return;
                    }

                    *is_updating_product = true;
                }

                let result = fetch_new_products().await;
                *use_application().is_updating_product() = false;

                if let Err(err) = result {
                    log::error!("Failed to fetch new products due to: {err:#?}");
                }
            });
        }
        "product/refresh-products-all" => {
            spawn(async {
                {
                    let mut is_updating_product = use_application().is_updating_product();

                    if *is_updating_product {
                        return;
                    }

                    *is_updating_product = true;
                }

                let result = refresh_products_all().await;
                *use_application().is_updating_product() = false;

                if let Err(err) = result {
                    log::error!("Failed to refresh products due to: {err:#?}");
                }
            });
        }
        "product/scan-downloaded-products" => {
            spawn(async {
                {
                    let mut is_updating_product = use_application().is_updating_product();

                    if *is_updating_product {
                        return;
                    }

                    *is_updating_product = true;
                }

                let result = scan_downloaded_products().await;
                *use_application().is_updating_product() = false;

                if let Err(err) = result {
                    log::error!("Failed to scan downloaded products due to: {err:#?}");
                }
            });
        }
        "setting/open-setting" => {
            SettingWindow.build_or_focus(use_application().app_handle())?;
        }
        "log/open-log-directory" => {
            let app_handle = use_application().app_handle();

            if let Ok(dir) = app_handle.path().app_log_dir() {
                app_handle
                    .opener()
                    .open_path(dir.to_str().unwrap(), None::<&str>)?;
            }
        }
        _ => {}
    }

    Ok(())
}
