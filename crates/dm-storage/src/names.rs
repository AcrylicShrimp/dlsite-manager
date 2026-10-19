use crate::{Result, StorageError};
use sqlx::{QueryBuilder, Sqlite};

pub const DEFAULT_LANGUAGE_PRIORITY: [&str; 5] = ["en_US", "ja_JP", "ko_KR", "zh_TW", "zh_CN"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguagePriority(Vec<String>);

impl Default for LanguagePriority {
    fn default() -> Self {
        Self(
            DEFAULT_LANGUAGE_PRIORITY
                .into_iter()
                .map(str::to_owned)
                .collect(),
        )
    }
}

impl LanguagePriority {
    pub fn new(codes: Vec<String>) -> Result<Self> {
        let mut seen = std::collections::BTreeSet::new();
        for code in &codes {
            if code.trim().is_empty() {
                return Err(StorageError::InvalidLanguagePriority(
                    "language code must not be blank",
                ));
            }
            if !seen.insert(code) {
                return Err(StorageError::InvalidLanguagePriority(
                    "language codes must be unique",
                ));
            }
        }
        Ok(Self(codes))
    }

    pub fn codes(&self) -> &[String] {
        &self.0
    }

    pub fn into_codes(self) -> Vec<String> {
        self.0
    }

    pub(super) fn to_json(&self) -> String {
        serde_json::to_string(&self.0).expect("strings serialize as JSON")
    }

    pub(super) fn from_stored(value: &str) -> Result<Self> {
        serde_json::from_str(value)
            .ok()
            .and_then(|codes| Self::new(codes).ok())
            .ok_or_else(|| StorageError::InvalidStoredValue {
                field: "app_settings.language_priority",
                value: value.to_owned(),
            })
    }
}

/// Computed name columns share the same selection rule across filters, sorting,
/// pagination and detail queries. The underlying works table stores only maps.
pub(super) fn product_query(priority: &LanguagePriority) -> QueryBuilder<Sqlite> {
    let mut builder = QueryBuilder::new(
        "WITH name_priority AS (
            SELECT value AS code, CAST(key AS INTEGER) AS rank FROM json_each(",
    );
    builder.push_bind(priority.to_json());
    builder.push(")), maker_identity_priority AS (SELECT value AS code, CAST(key AS INTEGER) AS rank FROM json_each(");
    builder.push_bind(LanguagePriority::default().to_json());
    builder.push(")), named_works AS (SELECT works.*, COALESCE(");
    push_preferred_name(&mut builder, "works.title_json", "name_priority");
    builder.push(", works.work_id) AS title, ");
    push_preferred_name(&mut builder, "works.maker_json", "name_priority");
    builder.push(" AS maker_name, CASE WHEN trim(COALESCE(works.maker_id, '')) <> '' THEN 'id:' || trim(works.maker_id) ELSE 'name:' || ");
    push_preferred_name(&mut builder, "works.maker_json", "maker_identity_priority");
    builder.push(" END AS maker_key FROM works) ");
    builder
}

fn push_preferred_name(builder: &mut QueryBuilder<Sqlite>, column: &str, priority_table: &str) {
    builder.push("(SELECT trim(name.value) FROM json_each(");
    push_valid_name_map(builder, column);
    builder
        .push(") AS name LEFT JOIN ")
        .push(priority_table)
        .push(" AS priority ON priority.code = name.key");
    builder.push(
        "
         WHERE name.type = 'text' AND trim(name.value) <> ''
         ORDER BY priority.rank IS NULL, priority.rank, name.key COLLATE BINARY
         LIMIT 1)",
    );
}

pub(super) fn push_valid_name_map(builder: &mut QueryBuilder<Sqlite>, column: &str) {
    // Only trusted column names from this module's callers are SQL text.
    builder
        .push("CASE WHEN json_valid(")
        .push(column)
        .push(") THEN CASE WHEN json_type(");
    builder.push(column).push(") = 'object' THEN ").push(column);
    builder.push(" ELSE '{}' END ELSE '{}' END");
}

pub(super) fn preferred_name(json: Option<&str>) -> Option<String> {
    let map: std::collections::BTreeMap<String, serde_json::Value> =
        serde_json::from_str(json?).ok()?;
    DEFAULT_LANGUAGE_PRIORITY
        .into_iter()
        .filter_map(|code| map.get(code))
        .chain(map.values())
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .find(|name| !name.is_empty())
        .map(str::to_owned)
}
