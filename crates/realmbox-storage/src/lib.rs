use std::path::Path;

use realmbox_domain::SetupSnapshot;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use thiserror::Error;

const MIGRATIONS: &[(&str, &str)] = &[(
    "0001_initial",
    "CREATE TABLE app_state (id INTEGER PRIMARY KEY CHECK (id = 1), snapshot_json TEXT NOT NULL, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);\n\
         CREATE TABLE sessions (id INTEGER PRIMARY KEY AUTOINCREMENT, started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, ended_at TEXT, outcome TEXT);",
)];

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("erreur SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("état persistant invalide: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub struct StateStore {
    connection: Connection,
}

impl StateStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        let mut store = Self { connection };
        store.migrate()?;
        Ok(store)
    }

    pub fn in_memory() -> Result<Self, StorageError> {
        let connection = Connection::open_in_memory()?;
        let mut store = Self { connection };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&mut self) -> Result<(), StorageError> {
        self.connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (version TEXT PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);",
        )?;
        for (version, sql) in MIGRATIONS {
            let tx = self.connection.transaction()?;
            let applied: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
                [version],
                |row| row.get(0),
            )?;
            if !applied {
                tx.execute_batch(sql)?;
                tx.execute(
                    "INSERT INTO schema_migrations(version) VALUES (?1)",
                    [version],
                )?;
            }
            tx.commit()?;
        }
        Ok(())
    }

    pub fn load(&self) -> Result<SetupSnapshot, StorageError> {
        let raw: Option<String> = self
            .connection
            .query_row(
                "SELECT snapshot_json FROM app_state WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        raw.map(|json| serde_json::from_str(&json))
            .transpose()
            .map(|snapshot| snapshot.unwrap_or_default())
            .map_err(StorageError::from)
    }

    pub fn save(&mut self, snapshot: &SetupSnapshot) -> Result<(), StorageError> {
        let tx = self.connection.transaction()?;
        save_in_transaction(&tx, snapshot)?;
        tx.commit()?;
        Ok(())
    }
}

fn save_in_transaction(tx: &Transaction<'_>, snapshot: &SetupSnapshot) -> Result<(), StorageError> {
    let json = serde_json::to_string(snapshot)?;
    tx.execute(
        "INSERT INTO app_state(id, snapshot_json) VALUES (1, ?1)\n\
         ON CONFLICT(id) DO UPDATE SET snapshot_json = excluded.snapshot_json, updated_at = CURRENT_TIMESTAMP",
        params![json],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use realmbox_domain::SetupState;

    #[test]
    fn persists_and_recovers_a_snapshot() {
        let mut store = StateStore::in_memory().expect("store");
        let mut snapshot = SetupSnapshot::default();
        snapshot
            .transition(SetupState::InspectingEnvironment, "setup.inspecting")
            .expect("transition");
        store.save(&snapshot).expect("save");
        assert_eq!(store.load().expect("load"), snapshot);
    }

    #[test]
    fn opens_unicode_path() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("état du monde.sqlite");
        let store = StateStore::open(&path).expect("open unicode sqlite path");
        assert_eq!(
            store.load().expect("load").state,
            realmbox_domain::SetupState::Uninitialized
        );
    }
}
