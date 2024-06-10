use crate::{
    application_error::{Error, Result},
    database::Database,
    window::{BuildableWindow, MainWindow},
};
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
use rusqlite::Connection;
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
    database: Mutex<Option<Database>>,
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

        let mut database = Database::load(app_dir.join("database.db"))?;
        rusqlite::vtab::array::load_module(database.connection_mut())?;

        Ok(Self {
            app_handle: app.handle(),
            database: Mutex::new(Some(database)),
            is_updating_product: Mutex::new(false),
        })
    }

    pub fn app_handle(&self) -> &AppHandle {
        &self.app_handle
    }

    pub fn connection(&self) -> MappedMutexGuard<Connection> {
        MutexGuard::map(self.database.lock(), |storage| {
            storage.as_mut().unwrap().connection_mut()
        })
    }

    pub fn is_updating_product(&self) -> MutexGuard<bool> {
        self.is_updating_product.lock()
    }

    pub fn init(&self) -> Result<()> {
        self.database.lock().as_ref().unwrap().prepare()?;
        Ok(())
    }

    pub fn run(&self) -> Result<()> {
        MainWindow.build(&self.app_handle)?;
        Ok(())
    }

    pub fn drop_storage(&self) -> Result<()> {
        if let Some(storage) = self.database.lock().take() {
            storage.drop()?;
        }
        Ok(())
    }
}
