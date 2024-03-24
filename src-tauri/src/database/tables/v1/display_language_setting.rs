use crate::{
    application::use_application, application_error::Result,
    database::models::v1::DisplayLanguageSetting,
};
use rusqlite::params;

pub struct DisplayLanguageSettingTable;

impl DisplayLanguageSettingTable {
    pub fn get_ddl() -> &'static str {
        "
CREATE TABLE IF NOT EXISTS display_language_settings (
    language TEXT PRIMARY KEY NOT NULL,
    order_index INTEGER NOT NULL UNIQUE
);"
    }

    pub fn get() -> Result<DisplayLanguageSetting> {
        let mut languages = use_application()
            .connection()
            .prepare(
                "
SELECT
    language
FROM display_language_settings
ORDER BY order_index ASC;",
            )?
            .query_map((), |row| row.get::<_, String>("language"))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        if languages.len() == 0 {
            languages = vec![
                "japanese".to_owned(),
                "english".to_owned(),
                "korean".to_owned(),
                "taiwanese".to_owned(),
                "chinese".to_owned(),
            ];
        }

        Ok(DisplayLanguageSetting { languages })
    }

    pub fn set(setting: &DisplayLanguageSetting) -> Result<()> {
        let mut connection = use_application().connection();
        let tx = connection.transaction()?;
        {
            tx.execute(
                "
    DELETE FROM display_language_settings",
                (),
            )?;

            let mut insert_stmt = tx.prepare(
                "
    INSERT INTO display_language_settings (
        language,
        order_index
    ) VALUES (
        ?1,
        ?2
    )",
            )?;

            for (index, language) in setting.languages.iter().enumerate() {
                insert_stmt.execute(params![language, index])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}
