//! `SQLite` storage backend for `IndexedDB`.
//!
//! Each origin gets a single `SQLite` database file. Meta-tables track
//! database names, versions, and object store definitions.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::IdbKey;

/// Schema for the meta-tables that track IDB databases and object stores.
const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS _idb_meta (
    db_name TEXT PRIMARY KEY,
    version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS _idb_stores (
    db_name    TEXT NOT NULL,
    store_name TEXT NOT NULL,
    key_path   TEXT,
    auto_increment INTEGER NOT NULL DEFAULT 0,
    next_key   INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (db_name, store_name)
);

CREATE TABLE IF NOT EXISTS _idb_indexes (
    db_name    TEXT NOT NULL,
    store_name TEXT NOT NULL,
    index_name TEXT NOT NULL,
    key_path   TEXT NOT NULL,
    is_unique  INTEGER NOT NULL DEFAULT 0,
    multi_entry INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (db_name, store_name, index_name)
);
";

/// Wrapper around a `rusqlite::Connection` providing IDB operations.
pub struct IdbBackend {
    conn: Connection,
}

/// Error type for backend operations.
///
/// Variant names match W3C `IndexedDB` 3.0 `DOMException` names.
#[derive(Debug)]
pub enum BackendError {
    /// Key already exists in a unique index or object store.
    ConstraintError(String),
    /// Object store or index not found.
    NotFoundError(String),
    /// Invalid key, key path, or key range.
    DataError(String),
    /// Write attempted in a read-only transaction.
    ReadOnlyError(String),
    /// Operation attempted on an inactive transaction.
    TransactionInactiveError(String),
    /// Version requested is lower than the existing version.
    VersionError(String),
    /// Invalid operation (e.g., autoIncrement + empty keyPath).
    InvalidAccessError(String),
    /// Transaction state does not allow the operation.
    InvalidStateError(String),
    /// `SQLite` or other internal error.
    Internal(String),
}

impl BackendError {
    /// Returns the W3C `DOMException` name for this error.
    pub fn dom_exception_name(&self) -> &'static str {
        match self {
            Self::ConstraintError(_) => "ConstraintError",
            Self::NotFoundError(_) => "NotFoundError",
            Self::DataError(_) => "DataError",
            Self::ReadOnlyError(_) => "ReadOnlyError",
            Self::TransactionInactiveError(_) => "TransactionInactiveError",
            Self::VersionError(_) => "VersionError",
            Self::InvalidAccessError(_) => "InvalidAccessError",
            Self::InvalidStateError(_) => "InvalidStateError",
            Self::Internal(_) => "UnknownError",
        }
    }
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConstraintError(msg)
            | Self::NotFoundError(msg)
            | Self::DataError(msg)
            | Self::ReadOnlyError(msg)
            | Self::TransactionInactiveError(msg)
            | Self::VersionError(msg)
            | Self::InvalidAccessError(msg)
            | Self::InvalidStateError(msg)
            | Self::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for BackendError {}

impl From<rusqlite::Error> for BackendError {
    fn from(e: rusqlite::Error) -> Self {
        // Sanitize: don't expose internal SQLite details to JS
        tracing::debug!("SQLite error: {e:#}");
        Self::Internal("internal storage error".into())
    }
}

use crate::util;

impl IdbBackend {
    /// Open or create a backend at the given file path.
    ///
    /// Configures WAL journal mode, 5s busy timeout, and `secure_delete`.
    pub fn open(path: &Path) -> Result<Self, BackendError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA busy_timeout = 5000;
             PRAGMA secure_delete = ON;",
        )?;
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(Self { conn })
    }

    /// Open an in-memory backend (for testing).
    pub fn open_in_memory() -> Result<Self, BackendError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA_SQL)?;
        Ok(Self { conn })
    }

    /// Returns a reference to the underlying connection (for transactions).
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Returns a mutable reference to the underlying connection.
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    // -- Database lifecycle --

    /// Get the version of a named database, or `None` if it doesn't exist.
    pub fn get_version(&self, db_name: &str) -> Result<Option<u64>, BackendError> {
        let result: Option<i64> = self
            .conn
            .query_row(
                "SELECT version FROM _idb_meta WHERE db_name = ?1",
                params![db_name],
                |row| row.get(0),
            )
            .optional()?;
        #[allow(clippy::cast_sign_loss)] // version stored as non-negative i64
        Ok(result.map(|v| v as u64))
    }

    /// Set the version of a named database (upsert).
    #[allow(clippy::cast_possible_wrap)]
    pub fn set_version(&self, db_name: &str, version: u64) -> Result<(), BackendError> {
        self.conn.execute(
            "INSERT INTO _idb_meta (db_name, version) VALUES (?1, ?2)
             ON CONFLICT(db_name) DO UPDATE SET version = excluded.version",
            params![db_name, version as i64],
        )?;
        Ok(())
    }

    /// Delete a database and all its object stores / indexes.
    pub fn delete_database(&self, db_name: &str) -> Result<(), BackendError> {
        // Drop all data tables
        let store_names = self.list_store_names(db_name)?;
        for store_name in &store_names {
            let table = util::data_table_name(db_name, store_name);
            // Drop index tables
            let index_names = self.list_index_names(db_name, store_name)?;
            for idx_name in &index_names {
                let idx_table = index_table_name(db_name, store_name, idx_name);
                self.conn
                    .execute(&format!("DROP TABLE IF EXISTS [{idx_table}]"), [])?;
            }
            self.conn
                .execute(&format!("DROP TABLE IF EXISTS [{table}]"), [])?;
        }
        self.conn.execute(
            "DELETE FROM _idb_indexes WHERE db_name = ?1",
            params![db_name],
        )?;
        self.conn.execute(
            "DELETE FROM _idb_stores WHERE db_name = ?1",
            params![db_name],
        )?;
        self.conn
            .execute("DELETE FROM _idb_meta WHERE db_name = ?1", params![db_name])?;
        Ok(())
    }

    /// List all database names in this origin.
    pub fn list_database_names(&self) -> Result<Vec<String>, BackendError> {
        let mut stmt = self.conn.prepare("SELECT db_name FROM _idb_meta")?;
        let names = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(names)
    }

    // -- Object store operations --

    /// Create an object store. Returns error if it already exists.
    pub fn create_object_store(
        &self,
        db_name: &str,
        store_name: &str,
        key_path: Option<&str>,
        auto_increment: bool,
    ) -> Result<(), BackendError> {
        /// Maximum object stores per database (prevent resource exhaustion).
        const MAX_STORES: i64 = 200;

        // Check count limit
        let store_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM _idb_stores WHERE db_name = ?1",
            params![db_name],
            |row| row.get(0),
        )?;
        if store_count >= MAX_STORES {
            return Err(BackendError::ConstraintError(format!(
                "maximum number of object stores ({MAX_STORES}) reached"
            )));
        }

        // Check for duplicate
        let exists: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM _idb_stores WHERE db_name = ?1 AND store_name = ?2",
            params![db_name, store_name],
            |row| row.get(0),
        )?;
        if exists {
            return Err(BackendError::ConstraintError(format!(
                "Object store '{store_name}' already exists"
            )));
        }

        self.conn.execute(
            "INSERT INTO _idb_stores (db_name, store_name, key_path, auto_increment) VALUES (?1, ?2, ?3, ?4)",
            params![db_name, store_name, key_path, i32::from(auto_increment)],
        )?;

        // Create data table: key_data BLOB (serialized IdbKey), value TEXT (JSON)
        let table = util::data_table_name(db_name, store_name);
        self.conn.execute_batch(&format!(
            "CREATE TABLE [{table}] (
                key_data BLOB NOT NULL PRIMARY KEY,
                value    TEXT NOT NULL
            )"
        ))?;

        Ok(())
    }

    /// Delete an object store and its data table.
    pub fn delete_object_store(&self, db_name: &str, store_name: &str) -> Result<(), BackendError> {
        let exists: bool = self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM _idb_stores WHERE db_name = ?1 AND store_name = ?2",
            params![db_name, store_name],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(BackendError::NotFoundError(format!(
                "Object store '{store_name}' not found"
            )));
        }

        // Drop index tables
        let index_names = self.list_index_names(db_name, store_name)?;
        for idx_name in &index_names {
            let idx_table = index_table_name(db_name, store_name, idx_name);
            self.conn
                .execute(&format!("DROP TABLE IF EXISTS [{idx_table}]"), [])?;
        }
        self.conn.execute(
            "DELETE FROM _idb_indexes WHERE db_name = ?1 AND store_name = ?2",
            params![db_name, store_name],
        )?;

        let table = util::data_table_name(db_name, store_name);
        self.conn
            .execute(&format!("DROP TABLE IF EXISTS [{table}]"), [])?;

        self.conn.execute(
            "DELETE FROM _idb_stores WHERE db_name = ?1 AND store_name = ?2",
            params![db_name, store_name],
        )?;

        Ok(())
    }

    /// Rename an object store (W3C §4.5 `IDBObjectStore.name` setter).
    pub fn rename_object_store(
        &self,
        db_name: &str,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), BackendError> {
        let old_table = util::data_table_name(db_name, old_name);
        let new_table = util::data_table_name(db_name, new_name);
        self.conn.execute(
            "UPDATE _idb_stores SET store_name = ?3 WHERE db_name = ?1 AND store_name = ?2",
            params![db_name, old_name, new_name],
        )?;
        self.conn.execute_batch(&format!(
            "ALTER TABLE [{old_table}] RENAME TO [{new_table}]"
        ))?;
        // Rename index backing tables
        let index_names = self.list_index_names(db_name, old_name)?;
        for idx_name in &index_names {
            let old_idx = util::index_table_name(db_name, old_name, idx_name);
            let new_idx = util::index_table_name(db_name, new_name, idx_name);
            self.conn
                .execute_batch(&format!("ALTER TABLE [{old_idx}] RENAME TO [{new_idx}]"))?;
        }
        // Update index metadata
        self.conn.execute(
            "UPDATE _idb_indexes SET store_name = ?3 WHERE db_name = ?1 AND store_name = ?2",
            params![db_name, old_name, new_name],
        )?;
        Ok(())
    }

    /// List object store names for a database, sorted alphabetically.
    pub fn list_store_names(&self, db_name: &str) -> Result<Vec<String>, BackendError> {
        let mut stmt = self
            .conn
            .prepare("SELECT store_name FROM _idb_stores WHERE db_name = ?1 ORDER BY store_name")?;
        let names = stmt
            .query_map(params![db_name], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(names)
    }

    /// Get store metadata: `(key_path, auto_increment)`.
    pub fn get_store_meta(
        &self,
        db_name: &str,
        store_name: &str,
    ) -> Result<(Option<String>, bool), BackendError> {
        let result = self
            .conn
            .query_row(
                "SELECT key_path, auto_increment FROM _idb_stores WHERE db_name = ?1 AND store_name = ?2",
                params![db_name, store_name],
                |row| {
                    let key_path: Option<String> = row.get(0)?;
                    let auto_inc: i32 = row.get(1)?;
                    Ok((key_path, auto_inc != 0))
                },
            )
            .optional()?;
        result.ok_or_else(|| {
            BackendError::NotFoundError(format!("Object store '{store_name}' not found"))
        })
    }

    /// Get and atomically increment the next auto-increment key for a store.
    ///
    /// W3C §2.11: If the current number exceeds 2^53, returns `ConstraintError`.
    pub fn next_auto_key(&self, db_name: &str, store_name: &str) -> Result<IdbKey, BackendError> {
        /// Maximum auto-increment value per W3C `IndexedDB` §2.11.
        const MAX_KEY: i64 = 1_i64 << 53; // 9007199254740992

        let current: i64 = self.conn.query_row(
            "SELECT next_key FROM _idb_stores WHERE db_name = ?1 AND store_name = ?2",
            params![db_name, store_name],
            |row| row.get(0),
        )?;
        if current > MAX_KEY {
            return Err(BackendError::ConstraintError(
                "auto-increment key generator overflow (> 2^53)".into(),
            ));
        }
        self.conn.execute(
            "UPDATE _idb_stores SET next_key = next_key + 1 WHERE db_name = ?1 AND store_name = ?2",
            params![db_name, store_name],
        )?;
        #[allow(clippy::cast_precision_loss)]
        Ok(IdbKey::Number(current as f64))
    }

    /// Update the next auto-increment key if the provided key is >= current.
    #[allow(clippy::cast_possible_truncation)]
    pub fn maybe_bump_auto_key(
        &self,
        db_name: &str,
        store_name: &str,
        key: &IdbKey,
    ) -> Result<(), BackendError> {
        if let IdbKey::Number(v) = key {
            let int_val = v.floor() as i64 + 1;
            self.conn.execute(
                "UPDATE _idb_stores SET next_key = MAX(next_key, ?3) WHERE db_name = ?1 AND store_name = ?2",
                params![db_name, store_name, int_val],
            )?;
        }
        Ok(())
    }

    /// Returns the data table name for a store (exposed for ops/cursor modules).
    pub fn data_table(&self, db_name: &str, store_name: &str) -> String {
        util::data_table_name(db_name, store_name)
    }

    // -- Index metadata helpers --

    /// List index names for a store.
    pub fn list_index_names(
        &self,
        db_name: &str,
        store_name: &str,
    ) -> Result<Vec<String>, BackendError> {
        let mut stmt = self.conn.prepare(
            "SELECT index_name FROM _idb_indexes WHERE db_name = ?1 AND store_name = ?2 ORDER BY index_name",
        )?;
        let names = stmt
            .query_map(params![db_name, store_name], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(names)
    }
}

fn index_table_name(db_name: &str, store_name: &str, index_name: &str) -> String {
    util::index_table_name(db_name, store_name, index_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_backend() -> IdbBackend {
        IdbBackend::open_in_memory().unwrap()
    }

    #[test]
    fn version_lifecycle() {
        let b = mem_backend();
        assert_eq!(b.get_version("testdb").unwrap(), None);

        b.set_version("testdb", 1).unwrap();
        assert_eq!(b.get_version("testdb").unwrap(), Some(1));

        b.set_version("testdb", 5).unwrap();
        assert_eq!(b.get_version("testdb").unwrap(), Some(5));
    }

    #[test]
    fn list_database_names() {
        let b = mem_backend();
        b.set_version("alpha", 1).unwrap();
        b.set_version("beta", 1).unwrap();
        let mut names = b.list_database_names().unwrap();
        names.sort();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn create_and_list_stores() {
        let b = mem_backend();
        b.set_version("db", 1).unwrap();
        b.create_object_store("db", "users", Some("id"), false)
            .unwrap();
        b.create_object_store("db", "posts", None, true).unwrap();

        let names = b.list_store_names("db").unwrap();
        assert_eq!(names, vec!["posts", "users"]); // alphabetical
    }

    #[test]
    fn create_duplicate_store_fails() {
        let b = mem_backend();
        b.set_version("db", 1).unwrap();
        b.create_object_store("db", "items", None, false).unwrap();
        let err = b.create_object_store("db", "items", None, false);
        assert!(matches!(err, Err(BackendError::ConstraintError(_))));
    }

    #[test]
    fn delete_store() {
        let b = mem_backend();
        b.set_version("db", 1).unwrap();
        b.create_object_store("db", "temp", None, false).unwrap();
        assert_eq!(b.list_store_names("db").unwrap().len(), 1);

        b.delete_object_store("db", "temp").unwrap();
        assert!(b.list_store_names("db").unwrap().is_empty());
    }

    #[test]
    fn delete_nonexistent_store_fails() {
        let b = mem_backend();
        let err = b.delete_object_store("db", "nope");
        assert!(matches!(err, Err(BackendError::NotFoundError(_))));
    }

    #[test]
    fn get_store_meta() {
        let b = mem_backend();
        b.set_version("db", 1).unwrap();
        b.create_object_store("db", "s1", Some("id"), true).unwrap();
        b.create_object_store("db", "s2", None, false).unwrap();

        let (kp, ai) = b.get_store_meta("db", "s1").unwrap();
        assert_eq!(kp.as_deref(), Some("id"));
        assert!(ai);

        let (kp, ai) = b.get_store_meta("db", "s2").unwrap();
        assert!(kp.is_none());
        assert!(!ai);
    }

    #[test]
    fn get_store_meta_not_found() {
        let b = mem_backend();
        let err = b.get_store_meta("db", "nope");
        assert!(matches!(err, Err(BackendError::NotFoundError(_))));
    }

    #[test]
    fn auto_increment_key() {
        let b = mem_backend();
        b.set_version("db", 1).unwrap();
        b.create_object_store("db", "s", None, true).unwrap();

        let k1 = b.next_auto_key("db", "s").unwrap();
        let k2 = b.next_auto_key("db", "s").unwrap();
        let k3 = b.next_auto_key("db", "s").unwrap();

        assert_eq!(k1, IdbKey::Number(1.0));
        assert_eq!(k2, IdbKey::Number(2.0));
        assert_eq!(k3, IdbKey::Number(3.0));
    }

    #[test]
    fn bump_auto_key() {
        let b = mem_backend();
        b.set_version("db", 1).unwrap();
        b.create_object_store("db", "s", None, true).unwrap();

        // Bump to 100
        b.maybe_bump_auto_key("db", "s", &IdbKey::Number(99.5))
            .unwrap();
        let k = b.next_auto_key("db", "s").unwrap();
        assert_eq!(k, IdbKey::Number(100.0));
    }

    #[test]
    fn delete_database_cleans_up() {
        let b = mem_backend();
        b.set_version("db", 1).unwrap();
        b.create_object_store("db", "s1", None, false).unwrap();
        b.create_object_store("db", "s2", None, false).unwrap();

        b.delete_database("db").unwrap();
        assert_eq!(b.get_version("db").unwrap(), None);
        assert!(b.list_store_names("db").unwrap().is_empty());
    }

    #[test]
    fn data_table_name_sanitized() {
        // ASCII-only alphanumeric names use the fast path (no hex encoding)
        let name = util::data_table_name("mydb", "items");
        assert_eq!(name, "store_mydb_items");

        // Non-alphanumeric characters trigger hex encoding (collision-free)
        let name = util::data_table_name("my-db", "user.data");
        assert_ne!(name, "store_my_db_user_data"); // old collision-prone format
                                                   // "my-db" and "my_db" should produce DIFFERENT table names
        let name_dash = util::data_table_name("my-db", "s");
        let name_under = util::data_table_name("my_db", "s");
        assert_ne!(name_dash, name_under);
    }
}
