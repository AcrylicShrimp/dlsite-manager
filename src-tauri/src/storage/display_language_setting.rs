use crate::{application::use_application, application_error::Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DisplayLanguageSetting {
    pub languages: Vec<String>,
}

impl DisplayLanguageSetting {
    pub fn get_ddl() -> &'static str {
        "
CREATE TABLE IF NOT EXISTS display_language_settings (
    language TEXT PRIMARY KEY NOT NULL,
    order_index INTEGER NOT NULL UNIQUE
);"
    }

    pub fn get() -> Result<Self> {
        let mut languages = use_application()
            .storage()
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

        Ok(Self { languages })
    }

    pub fn set(setting: &Self) -> Result<()> {
        let mut connection = use_application().storage().connection();
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
