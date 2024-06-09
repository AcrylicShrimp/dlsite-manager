use super::DBResult;
use crate::{
    application::use_application,
    database::{models::v2::Setting, tables::Table},
};
use rusqlite::OptionalExtension;
use serde_rusqlite::*;

pub struct SettingTable;

impl Table for SettingTable {
    fn get_ddl() -> &'static str {
        r#"
CREATE TABLE IF NOT EXISTS v2_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    download_root_dir TEXT
);
"#
    }
}

impl SettingTable {
    /// Inserts or updates the singleton setting in the database.
    pub fn insert(setting: &Setting) -> DBResult<()> {
        let connection = use_application().connection();
        let mut stmt = connection.prepare(
            r#"
INSERT INTO v2_settings (id, download_root_dir) VALUES (1, :download_root_dir)
ON CONFLICT(id) DO UPDATE SET
    download_root_dir = excluded.download_root_dir;
"#,
        )?;

        stmt.execute(to_params_named(setting)?.to_slice().as_slice())?;
        Ok(())
    }

    /// Gets the singleton setting from the database.
    pub fn get() -> DBResult<Option<Setting>> {
        let connection = use_application().connection();
        let mut stmt = connection.prepare(
            r#"
SELECT
    id, download_root_dir
FROM v2_settings
WHERE id = 1;
"#,
        )?;

        let setting = stmt
            .query_row([], |row| Ok(from_row::<Setting>(row)))
            .optional()?
            .transpose()?;
        Ok(setting)
    }
}
