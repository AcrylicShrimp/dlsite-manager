use tauri::{Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub trait WindowInfoProvider {
    fn label(&self) -> String;
    fn entry(&self) -> String;
    fn title(&self) -> String;
    fn size(&self) -> (f64, f64);
    fn resizable(&self) -> bool;
    fn init_scripts(&self) -> Vec<String> {
        Vec::new()
    }
}

pub trait BuildableWindow<R>
where
    R: Runtime,
{
    fn build<M: Manager<R>>(&self, manager: &M) -> Result<WebviewWindow<R>>;
    fn build_or_focus<M: Manager<R>>(&self, manager: &M) -> Result<WebviewWindow<R>>;
}

impl<R, T> BuildableWindow<R> for T
where
    R: Runtime,
    T: WindowInfoProvider,
{
    fn build<M: Manager<R>>(&self, manager: &M) -> Result<WebviewWindow<R>> {
        let (width, height) = self.size();
        let mut builder = WebviewWindowBuilder::new(
            manager,
            self.label(),
            WebviewUrl::App(<_>::from(&self.entry())),
        )
        .title(self.title())
        .inner_size(width, height)
        .resizable(self.resizable())
        .visible(false);

        for script in self.init_scripts() {
            builder = builder.initialization_script(&script);
        }

        let window = builder.build()?;
        Ok(window)
    }

    fn build_or_focus<M: Manager<R>>(&self, manager: &M) -> Result<WebviewWindow<R>> {
        if let Some(window) = manager.get_webview_window(&self.label()) {
            window.set_focus()?;
            Ok(window)
        } else {
            self.build(manager)
        }
    }
}

mod account_add_window;
mod account_edit_window;
mod account_management_window;
mod main_window;
mod setting_window;

pub use account_add_window::*;
pub use account_edit_window::*;
pub use account_management_window::*;
pub use main_window::*;
pub use setting_window::*;

use crate::application_error::Result;
