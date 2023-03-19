use crate::{
    application_error::{Error, Result},
    storage::Storage,
    window::{BuildableWindow, MainWindow},
};
use parking_lot::{Mutex, MutexGuard};
use std::{fs::create_dir_all, mem::MaybeUninit, sync::Arc};
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
    is_updating_product: Mutex<bool>,
}

impl Application {
    pub fn new(app: &App) -> Result<Self> {
        let app_dir = if let Some(app_dir) = app.path_resolver().app_config_dir() {
            app_dir
        } else {
            return Err(Error::AppDirNotExist);
        };

        create_dir_all(&app_dir).map_err(|err| Error::AppDirCreationError { io_error: err })?;

        Ok(Self {
            app_handle: app.handle(),
            storage: Storage::load(app_dir.join("database.db"))?,
            is_updating_product: Mutex::new(false),
        })
    }

    pub fn app_handle(&self) -> &AppHandle {
        &self.app_handle
    }

    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    pub fn is_updating_product(&self) -> MutexGuard<bool> {
        self.is_updating_product.lock()
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
