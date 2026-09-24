use rusqlite::{Connection, TransactionBehavior};
use std::{path::Path, time::Duration};

pub(crate) const SCHEMA_VERSION: u32 = 1;
pub(crate) const APPLICATION_ID: u32 = 0x4147_5132;

/// Initialization is serialized by SQLite itself, including cross-process opens.
/// A process-wide exclusive lock would prevent the supported competing writers.
pub(crate) fn open(path: &Path, new_repository_id: &str) -> Result<Connection, String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let mut connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|error| error.to_string())?;
    connection
        .execute_batch("PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;")
        .map_err(|error| error.to_string())?;
    {
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let version: u32 = transaction
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        let application: u32 = transaction
            .query_row("PRAGMA application_id", [], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        if version == 0 && application == 0 {
            let tables: u32 = transaction
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            if tables != 0 {
                return Err("refusing an unversioned nonempty database".into());
            }
            transaction
                .execute_batch(include_str!("schema.sql"))
                .map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "INSERT INTO repository_metadata(singleton, repository_id, format) VALUES(1, ?, 'agentique-modeling-sqlite/1')",
                    [new_repository_id],
                )
                .map_err(|error| error.to_string())?;
            transaction
                .pragma_update(None, "application_id", APPLICATION_ID)
                .map_err(|error| error.to_string())?;
            transaction
                .pragma_update(None, "user_version", SCHEMA_VERSION)
                .map_err(|error| error.to_string())?;
        } else if version != SCHEMA_VERSION || application != APPLICATION_ID {
            return Err(format!(
                "unsupported repository schema/application: version={version}, application={application}; expected version={SCHEMA_VERSION}, application={APPLICATION_ID}"
            ));
        }
        transaction.commit().map_err(|error| error.to_string())?;
    }
    let mode: String = connection
        .query_row("PRAGMA journal_mode=WAL", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(format!("repository requires SQLite WAL, obtained {mode}"));
    }
    let check: String = connection
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if check != "ok" {
        return Err(format!("SQLite integrity failure: {check}"));
    }
    Ok(connection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_full_wal_and_retains_repository_identity() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("model.db");
        let first = open(&path, "first").unwrap();
        let synchronous: u32 = first
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .unwrap();
        let foreign_keys: u32 = first
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!((synchronous, foreign_keys), (2, 1));
        let second = open(&path, "second").unwrap();
        let identity: String = second
            .query_row("SELECT repository_id FROM repository_metadata", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(identity, "first");
    }

    #[test]
    fn refuses_unknown_schemas_without_rewriting_version() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("model.db");
        let connection = open(&path, "first").unwrap();
        connection.pragma_update(None, "user_version", 99).unwrap();
        drop(connection);
        assert!(open(&path, "second").is_err());
        let connection = Connection::open(path).unwrap();
        let version: u32 = connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 99);
    }

    #[test]
    fn refuses_gen1_or_other_unversioned_database() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("model.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("CREATE TABLE objects(id TEXT);")
            .unwrap();
        drop(connection);
        assert!(open(&path, "second").is_err());
    }
}
