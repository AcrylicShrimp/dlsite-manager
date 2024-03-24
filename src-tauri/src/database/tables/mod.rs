pub mod v2;

/// Represents a table in the database.
pub trait Table {
    /// Returns the DDL for the table.
    fn get_ddl() -> &'static str;
}
