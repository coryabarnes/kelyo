//! The profile database (`kite.db`): history, the open tabs saved for the
//! next start, and per-site settings.
//!
//! One SQLite file in WAL mode, so the UI thread can write while the
//! `kite://history` page reads it from another connection.

use std::collections::HashSet;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

/// The schema, applied in order. Never edit a shipped entry; append a new one.
const MIGRATIONS: &[&str] = &["CREATE TABLE visits (
         url TEXT PRIMARY KEY,
         -- The URL without its scheme or a leading `www.`, for address bar completion.
         stripped TEXT NOT NULL,
         host TEXT NOT NULL,
         title TEXT NOT NULL DEFAULT '',
         visit_count INTEGER NOT NULL DEFAULT 0,
         last_visit INTEGER NOT NULL
     );
     CREATE INDEX visits_stripped ON visits (stripped);
     CREATE INDEX visits_host ON visits (host);
     CREATE INDEX visits_last_visit ON visits (last_visit);
     CREATE TABLE session_tabs (
         position INTEGER PRIMARY KEY,
         url TEXT NOT NULL,
         title TEXT NOT NULL,
         active INTEGER NOT NULL,
         -- Pane number (0-based) if the tab is in split view.
         pane INTEGER
     );
     CREATE TABLE session_state (key TEXT PRIMARY KEY, value INTEGER NOT NULL);
     CREATE TABLE blocking_exceptions (host TEXT PRIMARY KEY);"];

pub struct Store {
    pub(crate) conn: Connection,
}

impl Store {
    /// Opens (creating if needed) the database at `path`.
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(2))?;
        Self::with_connection(conn)
    }

    /// A throwaway database, for tests and for running without a profile.
    pub fn in_memory() -> Self {
        let conn = Connection::open_in_memory().expect("SQLite can always open in memory");
        Self::with_connection(conn).expect("migrations apply to an empty database")
    }

    fn with_connection(conn: Connection) -> rusqlite::Result<Self> {
        let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        for (index, migration) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(migration)?;
            tx.pragma_update(None, "user_version", index as i64 + 1)?;
            tx.commit()?;
        }
        Ok(Self { conn })
    }

    /// Hosts the user turned blocking off for.
    pub fn blocking_exceptions(&self) -> rusqlite::Result<HashSet<String>> {
        let mut statement = self.conn.prepare("SELECT host FROM blocking_exceptions")?;
        statement.query_map([], |row| row.get(0))?.collect()
    }

    pub fn set_blocking_exception(&self, host: &str, excepted: bool) -> rusqlite::Result<()> {
        if excepted {
            self.conn.execute(
                "INSERT OR IGNORE INTO blocking_exceptions (host) VALUES (?1)",
                [host],
            )?;
        } else {
            self.conn
                .execute("DELETE FROM blocking_exceptions WHERE host = ?1", [host])?;
        }
        Ok(())
    }

    pub(crate) fn state(&self, key: &str) -> rusqlite::Result<Option<i64>> {
        self.conn
            .query_row(
                "SELECT value FROM session_state WHERE key = ?1",
                [key],
                |row| row.get(0),
            )
            .optional()
    }

    pub(crate) fn set_state(&self, key: &str, value: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO session_state (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocking_exceptions_round_trip() {
        let store = Store::in_memory();
        store.set_blocking_exception("example.com", true).unwrap();
        store.set_blocking_exception("example.com", true).unwrap();
        store.set_blocking_exception("other.example", true).unwrap();
        store
            .set_blocking_exception("other.example", false)
            .unwrap();
        assert_eq!(
            store.blocking_exceptions().unwrap(),
            HashSet::from(["example.com".to_owned()])
        );
    }

    #[test]
    fn reopening_a_file_keeps_data_and_skips_applied_migrations() {
        let dir = std::env::temp_dir().join(format!("kite-store-test-{}", std::process::id()));
        let path = dir.join("kite.db");
        Store::open(&path)
            .unwrap()
            .set_blocking_exception("a.example", true)
            .unwrap();
        let store = Store::open(&path).unwrap();
        assert!(store.blocking_exceptions().unwrap().contains("a.example"));
        drop(store);
        let _ = std::fs::remove_dir_all(dir);
    }
}
