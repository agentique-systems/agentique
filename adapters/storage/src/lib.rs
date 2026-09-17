//! SQLite WAL, FULL synchronous transactions. Private persistence, never model interchange.
use agq_application::{Event, Store, Transaction};
use agq_model::*;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;
use std::path::Path;
pub struct SqliteStore {
    connection: Connection,
    _lock: std::fs::File,
}
fn db_error(e: impl std::fmt::Display) -> Error {
    Error::new("storage_error", e.to_string())
}
impl SqliteStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).map_err(db_error)?;
        }
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path.with_extension("lock"))
            .map_err(db_error)?;
        lock.try_lock().map_err(|_| {
            Error::new(
                "workspace_busy",
                "Workspace is already open by another Engine process",
            )
        })?;
        let connection = Connection::open(path).map_err(db_error)?;
        let version: u32 = connection
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(db_error)?;
        if version > 1 {
            return Err(Error::new(
                "unsupported_storage_version",
                format!("Workspace schema {version} is newer than supported schema 1"),
            ));
        }
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(db_error)?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS objects(kind TEXT NOT NULL,id TEXT NOT NULL,data TEXT NOT NULL,digest TEXT NOT NULL,PRIMARY KEY(kind,id)); CREATE TABLE IF NOT EXISTS events(sequence INTEGER PRIMARY KEY AUTOINCREMENT,data TEXT NOT NULL,digest TEXT NOT NULL); CREATE TABLE IF NOT EXISTS receipts(id TEXT PRIMARY KEY,digest TEXT NOT NULL,result TEXT NOT NULL); PRAGMA user_version=1;").map_err(db_error)?;
        let check: String = connection
            .query_row("PRAGMA quick_check", [], |r| r.get(0))
            .map_err(db_error)?;
        if check != "ok" {
            return Err(Error::new("corrupt_data", check));
        }
        Ok(Self {
            connection,
            _lock: lock,
        })
    }
}
fn checked(data: String, expected: String) -> Result<Value> {
    if digest(data.as_bytes()) != expected {
        return Err(Error::new(
            "corrupt_data",
            "Stored record checksum mismatch",
        ));
    }
    serde_json::from_str(&data).map_err(db_error)
}
impl Store for SqliteStore {
    fn latest_sequence(&self) -> Result<u64> {
        self.connection
            .query_row("SELECT COALESCE(MAX(sequence),0) FROM events", [], |r| {
                r.get(0)
            })
            .map_err(db_error)
    }
    fn get(&self, kind: &str, id: &str) -> Result<Option<Value>> {
        let row: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT data,digest FROM objects WHERE kind=? AND id=?",
                params![kind, id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(db_error)?;
        row.map(|(d, h)| checked(d, h)).transpose()
    }
    fn list(&self, kind: &str) -> Result<Vec<Value>> {
        let mut q = self
            .connection
            .prepare("SELECT data,digest FROM objects WHERE kind=? ORDER BY rowid")
            .map_err(db_error)?;
        let rows = q
            .query_map([kind], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(db_error)?;
        rows.map(|r| {
            let (d, h) = r.map_err(db_error)?;
            checked(d, h)
        })
        .collect()
    }
    fn transact(&mut self, mut batch: Transaction) -> Result<u64> {
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db_error)?;
        for w in batch.writes {
            let data = serde_json::to_string(&w.value).map_err(db_error)?;
            tx.execute("INSERT INTO objects(kind,id,data,digest) VALUES(?,?,?,?) ON CONFLICT(kind,id) DO UPDATE SET data=excluded.data,digest=excluded.digest",params![w.kind,w.id,data,digest(&data)]).map_err(db_error)?;
        }
        let seq: u64 = tx
            .query_row("SELECT COALESCE(MAX(sequence),0)+1 FROM events", [], |r| {
                r.get(0)
            })
            .map_err(db_error)?;
        batch.event.sequence = seq;
        let data = serde_json::to_string(&batch.event).map_err(db_error)?;
        tx.execute(
            "INSERT INTO events(sequence,data,digest) VALUES(?,?,?)",
            params![seq, data, digest(&data)],
        )
        .map_err(db_error)?;
        if let Some((key, digest, result)) = batch.receipt {
            tx.execute(
                "INSERT INTO receipts(id,digest,result) VALUES(?,?,?)",
                params![
                    key,
                    digest,
                    serde_json::to_string(&result).map_err(db_error)?
                ],
            )
            .map_err(db_error)?;
        }
        tx.commit().map_err(db_error)?;
        Ok(seq)
    }
    fn events(&self, after: u64, limit: usize) -> Result<Vec<Event>> {
        let mut q = self
            .connection
            .prepare("SELECT data,digest FROM events WHERE sequence>? ORDER BY sequence LIMIT ?")
            .map_err(db_error)?;
        let rows = q
            .query_map(params![after, limit.min(i64::MAX as usize) as i64], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(db_error)?;
        rows.map(|r| {
            let (d, h) = r.map_err(db_error)?;
            serde_json::from_value(checked(d, h)?).map_err(db_error)
        })
        .collect()
    }
    fn receipt(&self, key: &str) -> Result<Option<(String, Value)>> {
        let row: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT digest,result FROM receipts WHERE id=?",
                [key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(db_error)?;
        row.map(|(d, r)| Ok((d, serde_json::from_str(&r).map_err(db_error)?)))
            .transpose()
    }
}
