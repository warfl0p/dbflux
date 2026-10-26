//! Migration 050: Add `blank_line_delimits_statements` to
//! `cfg_general_settings`.
//!
//! Persists whether a blank line ends a SQL statement in the editor. The
//! column defaults to 0 (off), the behavior before the setting existed.

use rusqlite::Transaction;

use crate::migrations::{Migration, MigrationError};

pub(crate) struct MigrationImpl;

impl Migration for MigrationImpl {
    fn name(&self) -> &str {
        "050_general_settings_blank_line_delimiter"
    }

    fn run(&self, tx: &Transaction) -> Result<(), MigrationError> {
        let table_exists: bool = tx
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='cfg_general_settings'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|n| n > 0)?;

        if !table_exists {
            return Ok(());
        }

        let column_exists: bool = tx
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('cfg_general_settings') WHERE name = 'blank_line_delimits_statements'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|n| n > 0)?;

        if !column_exists {
            tx.execute_batch(
                "ALTER TABLE cfg_general_settings ADD COLUMN blank_line_delimits_statements INTEGER NOT NULL DEFAULT 0;",
            )?;
        }

        Ok(())
    }
}
