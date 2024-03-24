use crate::{
    application::use_application, application_error::Result, database::models::v1::Setting,
};
use rusqlite::{params, OptionalExtension};

pub struct SettingTable;

impl SettingTable {
    pub fn get_ddl() -> &'static str {
        "
CREATE TABLE IF NOT EXISTS settings (
    download_root_dir TEXT
);"
    }

    pub fn get() -> Result<Setting> {
        Ok(use_application()
            .connection()
            .prepare(
                "
SELECT
    download_root_dir
FROM settings;",
            )?
            .query_row((), |row| Setting::try_from(row))
            .optional()?
            .unwrap_or_default())
    }

    pub fn set(setting: Setting) -> Result<()> {
        let connection = use_application().connection();
        connection.execute(
            "
DELETE FROM settings",
            (),
        )?;
        connection
            .prepare(
                "
INSERT INTO settings (
    download_root_dir
) VALUES (
    ?1
)",
            )?
            .insert(params![setting
                .download_root_dir
                .as_ref()
                .map(|path| path.to_str().unwrap())])?;

        Ok(())
    }
}
