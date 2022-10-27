use tauri::{Manager, Runtime, Window, WindowBuilder, WindowUrl};

pub trait WindowInfoProvider {
    fn label(&self) -> String;
    fn entry(&self) -> String;
    fn title(&self) -> String;
    fn size(&self) -> (f64, f64);
    fn resizable(&self) -> bool;
}

pub trait BuildableWindow<R>
where
    R: Runtime,
{
    fn build<'m, M: Manager<R>>(&self, manager: &'m M) -> Result<Window<R>>;
    fn build_or_focus<'m, M: Manager<R>>(&self, manager: &'m M) -> Result<Window<R>>;
}

impl<R, T> BuildableWindow<R> for T
where
    R: Runtime,
    T: WindowInfoProvider,
{
    fn build<'m, M: Manager<R>>(&self, manager: &'m M) -> Result<Window<R>> {
        let (width, height) = self.size();

        let window = WindowBuilder::new(
            manager,
            &self.label(),
            WindowUrl::App(<_>::from(&self.entry())),
        )
        .title(self.title())
        .inner_size(width, height)
        .resizable(self.resizable())
        .visible(false)
        .build()?;

        Ok(window)
    }

    fn build_or_focus<'m, M: Manager<R>>(&self, manager: &'m M) -> Result<Window<R>> {
        if let Some(window) = manager.get_window(&self.label()) {
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

pub use account_add_window::*;
pub use account_edit_window::*;
pub use account_management_window::*;
pub use main_window::*;

use crate::application_error::Result;
