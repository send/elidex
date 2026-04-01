use std::path::Path;

use rusqlite::Connection;

use crate::backend::{
    Migration, OpenOptions, StorageBackend, StorageConnection, StorageOp, StorageResult,
};
use crate::error::StorageError;

/// SQLite-based storage backend (Ch.22).
///
/// Provides the default storage engine for all elidex storage needs.
/// Configuration follows Ch.22 recommendations:
/// - journal_mode = WAL
/// - synchronous = NORMAL
/// - secure_delete = ON
/// - foreign_keys = ON
/// - busy_timeout = 5000ms
/// - cache_size = -8000 (8MB)
#[derive(Debug)]
pub struct SqliteBackend;

impl SqliteBackend {
    pub fn new() -> Self {
        Self
    }

    fn apply_pragmas(conn: &Connection, options: &OpenOptions) -> Result<(), StorageError> {
        let timeout_ms = options.busy_timeout.as_millis();
        let journal_mode = if options.wal_mode { "WAL" } else { "DELETE" };

        conn.execute_batch(&format!(
            "PRAGMA journal_mode = {journal_mode};\
             PRAGMA synchronous = NORMAL;\
             PRAGMA secure_delete = ON;\
             PRAGMA foreign_keys = ON;\
             PRAGMA busy_timeout = {timeout_ms};\
             PRAGMA cache_size = -8000;"
        ))?;

        Ok(())
    }
}

impl Default for SqliteBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for SqliteBackend {
    type Connection = SqliteConnection;

    fn open(&self, path: &Path, options: OpenOptions) -> Result<SqliteConnection, StorageError> {
        let flags = if options.read_only {
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX
        } else {
            let mut flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX;
            if options.create_if_missing {
                flags |= rusqlite::OpenFlags::SQLITE_OPEN_CREATE;
            }
            flags
        };

        if options.create_if_missing {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    StorageError::new(
                        crate::error::StorageErrorKind::Io,
                        format!("failed to create directory {}: {e}", parent.display()),
                    )
                })?;
            }
        }

        let conn = Connection::open_with_flags(path, flags)?;
        Self::apply_pragmas(&conn, &options)?;

        Ok(SqliteConnection { conn })
    }

    fn migrate(
        &self,
        connection: &SqliteConnection,
        migrations: &[Migration],
    ) -> Result<(), StorageError> {
        let conn = &connection.conn;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _schema_version (\
                 version INTEGER PRIMARY KEY\
             )",
        )?;

        let current: u32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM _schema_version",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        for m in migrations {
            if m.version > current {
                conn.execute_batch(m.sql).map_err(|e| {
                    StorageError::new(
                        crate::error::StorageErrorKind::Migration,
                        format!("migration v{} failed: {e}", m.version),
                    )
                })?;
                conn.execute(
                    "INSERT INTO _schema_version (version) VALUES (?1)",
                    [m.version],
                )?;
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "sqlite"
    }
}

/// A connection to a SQLite database.
pub struct SqliteConnection {
    conn: Connection,
}

impl SqliteConnection {
    /// Create an in-memory connection (for testing).
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        SqliteBackend::apply_pragmas(
            &conn,
            &OpenOptions {
                wal_mode: false, // WAL not supported for in-memory
                ..OpenOptions::default()
            },
        )?;
        Ok(Self { conn })
    }
}

impl std::fmt::Debug for SqliteConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteConnection").finish_non_exhaustive()
    }
}

impl StorageConnection for SqliteConnection {
    fn execute(&self, op: &StorageOp) -> Result<StorageResult, StorageError> {
        match op {
            StorageOp::Get { table, key } => {
                validate_table_name(table)?;
                let sql = format!("SELECT value FROM [{table}] WHERE key = ?1");
                match self.conn.query_row(&sql, [key], |row| {
                    let value: Vec<u8> = row.get(0)?;
                    Ok(value)
                }) {
                    Ok(value) => Ok(StorageResult::Row(value)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => {
                        Err(StorageError::not_found(format!(
                            "key not found in table [{table}]"
                        )))
                    }
                    Err(e) => Err(e.into()),
                }
            }

            StorageOp::Put { table, key, value } => {
                validate_table_name(table)?;
                let sql =
                    format!("INSERT OR REPLACE INTO [{table}] (key, value) VALUES (?1, ?2)");
                self.conn.execute(&sql, rusqlite::params![key, value])?;
                Ok(StorageResult::Ok)
            }

            StorageOp::Delete { table, key } => {
                validate_table_name(table)?;
                let sql = format!("DELETE FROM [{table}] WHERE key = ?1");
                let count = self.conn.execute(&sql, [key])?;
                Ok(StorageResult::Count(count))
            }

            StorageOp::Scan {
                table,
                prefix,
                limit,
            } => {
                validate_table_name(table)?;
                if prefix.is_empty() {
                    let sql = format!("SELECT value FROM [{table}] ORDER BY key LIMIT ?1");
                    let mut stmt = self.conn.prepare(&sql)?;
                    let rows: Vec<Vec<u8>> = stmt
                        .query_map([*limit as i64], |row| row.get(0))?
                        .collect::<Result<_, _>>()?;
                    Ok(StorageResult::Rows(rows))
                } else {
                    // Prefix scan: key >= prefix AND key < prefix_upper_bound
                    let upper = prefix_upper_bound(prefix);
                    let sql = format!(
                        "SELECT value FROM [{table}] WHERE key >= ?1 AND key < ?2 ORDER BY key LIMIT ?3"
                    );
                    let mut stmt = self.conn.prepare(&sql)?;
                    let rows: Vec<Vec<u8>> = stmt
                        .query_map(
                            rusqlite::params![prefix, upper, *limit as i64],
                            |row| row.get(0),
                        )?
                        .collect::<Result<_, _>>()?;
                    Ok(StorageResult::Rows(rows))
                }
            }

            StorageOp::Custom(op) => op.execute(&self.conn),
        }
    }

    fn transaction<F, T>(&self, f: F) -> Result<T, StorageError>
    where
        F: FnOnce(&Self) -> Result<T, StorageError>,
    {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        match f(self) {
            Ok(result) => {
                self.conn.execute_batch("COMMIT")?;
                Ok(result)
            }
            Err(e) => {
                let _ = self.conn.execute_batch("ROLLBACK");
                Err(e)
            }
        }
    }

    fn raw_connection(&self) -> &Connection {
        &self.conn
    }
}

/// Validate table name to prevent SQL injection.
fn validate_table_name(name: &str) -> Result<(), StorageError> {
    if name.is_empty()
        || name.len() > 128
        || name
            .chars()
            .any(|c| !c.is_alphanumeric() && c != '_' && c != '-')
    {
        return Err(StorageError::new(
            crate::error::StorageErrorKind::Other,
            format!("invalid table name: {name}"),
        ));
    }
    Ok(())
}

/// Compute the upper bound for prefix scanning.
///
/// Increments the last byte of the prefix. If overflow, truncates.
fn prefix_upper_bound(prefix: &[u8]) -> Vec<u8> {
    let mut upper = prefix.to_vec();
    while let Some(last) = upper.last_mut() {
        if *last < 0xFF {
            *last += 1;
            return upper;
        }
        upper.pop();
    }
    // All 0xFF — return empty (no upper bound, but this is an edge case)
    vec![0xFF]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{Migration, StorageOp};

    fn setup() -> SqliteConnection {
        let conn = SqliteConnection::open_in_memory().unwrap();
        conn.conn
            .execute_batch("CREATE TABLE [test] (key BLOB PRIMARY KEY, value BLOB NOT NULL)")
            .unwrap();
        conn
    }

    #[test]
    fn put_and_get() {
        let conn = setup();
        let put = StorageOp::Put {
            table: "test",
            key: b"hello",
            value: b"world",
        };
        conn.execute(&put).unwrap();

        let get = StorageOp::Get {
            table: "test",
            key: b"hello",
        };
        match conn.execute(&get).unwrap() {
            StorageResult::Row(v) => assert_eq!(v, b"world"),
            other => panic!("expected Row, got {other:?}"),
        }
    }

    #[test]
    fn get_not_found() {
        let conn = setup();
        let get = StorageOp::Get {
            table: "test",
            key: b"missing",
        };
        assert!(conn.execute(&get).is_err());
    }

    #[test]
    fn put_overwrites() {
        let conn = setup();
        conn.execute(&StorageOp::Put {
            table: "test",
            key: b"k",
            value: b"v1",
        })
        .unwrap();
        conn.execute(&StorageOp::Put {
            table: "test",
            key: b"k",
            value: b"v2",
        })
        .unwrap();

        match conn
            .execute(&StorageOp::Get {
                table: "test",
                key: b"k",
            })
            .unwrap()
        {
            StorageResult::Row(v) => assert_eq!(v, b"v2"),
            other => panic!("expected Row, got {other:?}"),
        }
    }

    #[test]
    fn delete() {
        let conn = setup();
        conn.execute(&StorageOp::Put {
            table: "test",
            key: b"k",
            value: b"v",
        })
        .unwrap();

        match conn
            .execute(&StorageOp::Delete {
                table: "test",
                key: b"k",
            })
            .unwrap()
        {
            StorageResult::Count(1) => {}
            other => panic!("expected Count(1), got {other:?}"),
        }

        assert!(conn
            .execute(&StorageOp::Get {
                table: "test",
                key: b"k",
            })
            .is_err());
    }

    #[test]
    fn delete_nonexistent() {
        let conn = setup();
        match conn
            .execute(&StorageOp::Delete {
                table: "test",
                key: b"nope",
            })
            .unwrap()
        {
            StorageResult::Count(0) => {}
            other => panic!("expected Count(0), got {other:?}"),
        }
    }

    #[test]
    fn scan_all() {
        let conn = setup();
        for i in 0..5u8 {
            conn.execute(&StorageOp::Put {
                table: "test",
                key: &[i],
                value: &[i + 10],
            })
            .unwrap();
        }

        match conn
            .execute(&StorageOp::Scan {
                table: "test",
                prefix: b"",
                limit: 100,
            })
            .unwrap()
        {
            StorageResult::Rows(rows) => assert_eq!(rows.len(), 5),
            other => panic!("expected Rows, got {other:?}"),
        }
    }

    #[test]
    fn scan_with_limit() {
        let conn = setup();
        for i in 0..5u8 {
            conn.execute(&StorageOp::Put {
                table: "test",
                key: &[i],
                value: &[i],
            })
            .unwrap();
        }

        match conn
            .execute(&StorageOp::Scan {
                table: "test",
                prefix: b"",
                limit: 3,
            })
            .unwrap()
        {
            StorageResult::Rows(rows) => assert_eq!(rows.len(), 3),
            other => panic!("expected Rows, got {other:?}"),
        }
    }

    #[test]
    fn scan_with_prefix() {
        let conn = setup();
        // Keys: "a1", "a2", "b1"
        conn.execute(&StorageOp::Put {
            table: "test",
            key: b"a1",
            value: b"v1",
        })
        .unwrap();
        conn.execute(&StorageOp::Put {
            table: "test",
            key: b"a2",
            value: b"v2",
        })
        .unwrap();
        conn.execute(&StorageOp::Put {
            table: "test",
            key: b"b1",
            value: b"v3",
        })
        .unwrap();

        match conn
            .execute(&StorageOp::Scan {
                table: "test",
                prefix: b"a",
                limit: 100,
            })
            .unwrap()
        {
            StorageResult::Rows(rows) => assert_eq!(rows.len(), 2),
            other => panic!("expected Rows, got {other:?}"),
        }
    }

    #[test]
    fn transaction_commit() {
        let conn = setup();
        conn.transaction(|c| {
            c.execute(&StorageOp::Put {
                table: "test",
                key: b"tk",
                value: b"tv",
            })?;
            Ok(())
        })
        .unwrap();

        assert!(conn
            .execute(&StorageOp::Get {
                table: "test",
                key: b"tk",
            })
            .is_ok());
    }

    #[test]
    fn transaction_rollback() {
        let conn = setup();
        let result: Result<(), StorageError> = conn.transaction(|c| {
            c.execute(&StorageOp::Put {
                table: "test",
                key: b"rk",
                value: b"rv",
            })?;
            Err(StorageError::new(
                crate::error::StorageErrorKind::Other,
                "intentional",
            ))
        });
        assert!(result.is_err());

        assert!(conn
            .execute(&StorageOp::Get {
                table: "test",
                key: b"rk",
            })
            .is_err());
    }

    #[test]
    fn invalid_table_name() {
        let conn = setup();
        assert!(conn
            .execute(&StorageOp::Get {
                table: "'; DROP TABLE test; --",
                key: b"k",
            })
            .is_err());
    }

    #[test]
    fn migration() {
        let conn = SqliteConnection::open_in_memory().unwrap();
        let backend = SqliteBackend::new();

        let migrations = [
            Migration {
                version: 1,
                sql: "CREATE TABLE [items] (key BLOB PRIMARY KEY, value BLOB NOT NULL)",
            },
            Migration {
                version: 2,
                sql: "CREATE TABLE [meta] (key BLOB PRIMARY KEY, value BLOB NOT NULL)",
            },
        ];

        backend.migrate(&conn, &migrations).unwrap();

        // Verify tables exist
        conn.execute(&StorageOp::Put {
            table: "items",
            key: b"k",
            value: b"v",
        })
        .unwrap();
        conn.execute(&StorageOp::Put {
            table: "meta",
            key: b"k",
            value: b"v",
        })
        .unwrap();

        // Re-run migrations (should be idempotent)
        backend.migrate(&conn, &migrations).unwrap();
    }

    #[test]
    fn migration_incremental() {
        let conn = SqliteConnection::open_in_memory().unwrap();
        let backend = SqliteBackend::new();

        let v1 = [Migration {
            version: 1,
            sql: "CREATE TABLE [t1] (key BLOB PRIMARY KEY, value BLOB NOT NULL)",
        }];
        backend.migrate(&conn, &v1).unwrap();

        let v1_v2 = [
            Migration {
                version: 1,
                sql: "CREATE TABLE [t1] (key BLOB PRIMARY KEY, value BLOB NOT NULL)",
            },
            Migration {
                version: 2,
                sql: "CREATE TABLE [t2] (key BLOB PRIMARY KEY, value BLOB NOT NULL)",
            },
        ];
        backend.migrate(&conn, &v1_v2).unwrap();

        conn.execute(&StorageOp::Put {
            table: "t2",
            key: b"k",
            value: b"v",
        })
        .unwrap();
    }

    #[test]
    fn prefix_upper_bound_basic() {
        assert_eq!(prefix_upper_bound(b"abc"), b"abd");
        assert_eq!(prefix_upper_bound(b"a"), b"b");
        assert_eq!(prefix_upper_bound(b"\x00"), b"\x01");
    }

    #[test]
    fn prefix_upper_bound_overflow() {
        assert_eq!(prefix_upper_bound(b"\xff"), vec![0xFF]);
        assert_eq!(prefix_upper_bound(b"a\xff"), b"b");
    }
}
