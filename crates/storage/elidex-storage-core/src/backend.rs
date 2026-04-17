use std::path::Path;
use std::time::Duration;

use crate::error::StorageError;

/// Options for opening a storage connection.
#[derive(Debug, Clone)]
pub struct OpenOptions {
    pub read_only: bool,
    pub create_if_missing: bool,
    pub wal_mode: bool,
    pub busy_timeout: Duration,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self {
            read_only: false,
            create_if_missing: true,
            wal_mode: true,
            busy_timeout: Duration::from_secs(5),
        }
    }
}

/// A database schema migration step.
#[derive(Debug, Clone)]
pub struct Migration {
    /// Monotonically increasing version number (1-based).
    pub version: u32,
    /// SQL to execute for this migration.
    pub sql: &'static str,
}

/// Result of a storage operation.
#[derive(Debug)]
pub enum StorageResult {
    /// No data returned (successful write/delete).
    Ok,
    /// Single row returned.
    Row(Vec<u8>),
    /// Multiple rows returned.
    Rows(Vec<Vec<u8>>),
    /// Number of affected rows.
    Count(usize),
}

/// Low-level storage operation (Ch.22 StorageOp).
///
/// Domain-level operations are built on top of these primitives.
/// The `Custom` variant provides an escape hatch for complex queries.
pub enum StorageOp<'a> {
    Get {
        table: &'a str,
        key: &'a [u8],
    },
    Put {
        table: &'a str,
        key: &'a [u8],
        value: &'a [u8],
    },
    Delete {
        table: &'a str,
        key: &'a [u8],
    },
    Scan {
        table: &'a str,
        prefix: &'a [u8],
        limit: usize,
    },
    Custom(Box<dyn CustomOp + 'a>),
}

/// Trait for custom storage operations that cannot be expressed
/// as simple Get/Put/Delete/Scan.
pub trait CustomOp {
    /// Execute the custom operation against a raw SQLite connection.
    fn execute(&self, conn: &rusqlite::Connection) -> Result<StorageResult, StorageError>;
}

/// Abstract storage backend (Ch.22 StorageBackend trait).
///
/// Concrete implementations wrap a specific database engine.
/// Currently only `SqliteBackend` is provided.
pub trait StorageBackend: Send + Sync {
    type Connection: StorageConnection;

    /// Open a connection to the database at the given path.
    fn open(&self, path: &Path, options: OpenOptions) -> Result<Self::Connection, StorageError>;

    /// Run schema migrations on an open connection.
    fn migrate(
        &self,
        conn: &Self::Connection,
        migrations: &[Migration],
    ) -> Result<(), StorageError>;

    /// Backend name (e.g., "sqlite").
    fn name(&self) -> &str;
}

/// Abstract storage connection (Ch.22 StorageConnection trait).
pub trait StorageConnection: Send {
    /// Execute a single storage operation.
    fn execute(&self, op: &StorageOp) -> Result<StorageResult, StorageError>;

    /// Run a closure within a database transaction.
    fn transaction<F, T>(&self, f: F) -> Result<T, StorageError>
    where
        F: FnOnce(&Self) -> Result<T, StorageError>;

    /// Access the underlying raw SQLite connection.
    ///
    /// Provided for `CustomOp` implementations and migration logic.
    /// Prefer `execute()` for standard operations.
    fn raw_connection(&self) -> &rusqlite::Connection;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_options_default() {
        let opts = OpenOptions::default();
        assert!(!opts.read_only);
        assert!(opts.create_if_missing);
        assert!(opts.wal_mode);
        assert_eq!(opts.busy_timeout, Duration::from_secs(5));
    }

    #[test]
    fn migration_ordering() {
        let m1 = Migration {
            version: 1,
            sql: "CREATE TABLE t1 (id INTEGER PRIMARY KEY)",
        };
        let m2 = Migration {
            version: 2,
            sql: "CREATE TABLE t2 (id INTEGER PRIMARY KEY)",
        };
        assert!(m1.version < m2.version);
    }
}
