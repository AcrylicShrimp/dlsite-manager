use tauri::{generate_handler, Builder, Runtime};

mod account_management;
mod product;
mod window;

pub trait CommandProvider<R>
where
    R: Runtime,
{
    fn attach_commands(self) -> Self;
}

impl<R> CommandProvider<R> for Builder<R>
where
    R: Runtime,
{
    fn attach_commands(self) -> Self {
        self.invoke_handler(generate_handler![
            account_management::account_management_list_accounts,
            account_management::account_management_get_account,
            account_management::account_management_add_account,
            account_management::account_management_update_account,
            account_management::account_management_remove_account,
            account_management::account_management_test_account,
            product::product_list_products,
            product::product_update_products,
            window::show_window,
            window::spawn_window_account_add,
            window::spawn_window_account_edit,
        ])
    }
}
