use crate::{
    application_error::{Error, Result},
    storage::Storage,
    window::{BuildableWindow, MainWindow},
};
use std::{mem::MaybeUninit, sync::Arc};
use tauri::{App, AppHandle};

static mut APPLICATION: MaybeUninit<Arc<Application>> = MaybeUninit::uninit();

pub fn use_application() -> &'static Application {
    unsafe { APPLICATION.assume_init_ref() }.as_ref()
}

pub fn create_application(app: &App) -> Result<Arc<Application>> {
    let application = Arc::new(Application::new(app)?);

    unsafe {
        APPLICATION.write(application.clone());
    }

    Ok(application)
}

pub struct Application {
    app_handle: AppHandle,
    storage: Storage,
}

impl Application {
    pub fn new(app: &App) -> Result<Self> {
        let app_dir = if let Some(app_dir) = app.path_resolver().app_dir() {
            app_dir
        } else {
            return Err(Error::AppDirNotExist);
        };

        Ok(Self {
            app_handle: app.handle(),
            storage: Storage::load(app_dir.join("database.db"))?,
        })
    }

    pub fn app_handle(&self) -> &AppHandle {
        &self.app_handle
    }

    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    pub fn init(&self) -> Result<()> {
        self.storage.prepare()?;
        Ok(())
    }

    pub fn run(&self) -> Result<()> {
        MainWindow.build(&self.app_handle)?;
        Ok(())
    }
}
