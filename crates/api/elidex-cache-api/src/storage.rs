//! CacheStorage operations (WHATWG Cache API §2.2).
//!
//! Manages named caches within an origin's storage.

use elidex_storage_core::{SqliteConnection, StorageConnection};

use crate::error::CacheError;

/// Schema migration for the cache names registry.
const CACHE_NAMES_SCHEMA: &str =
    "CREATE TABLE IF NOT EXISTS _cache_names (name TEXT PRIMARY KEY, created_at INTEGER NOT NULL)";

/// Ensure the cache names table exists.
fn ensure_names_table(conn: &SqliteConnection) -> Result<(), CacheError> {
    conn.raw_connection()
        .execute_batch(CACHE_NAMES_SCHEMA)
        .map_err(|e| CacheError::Storage(elidex_storage_core::StorageError::from(e)))?;
    Ok(())
}

/// Open (or create) a named cache.
///
/// If the cache doesn't exist, it is created and registered.
/// Returns `true` if the cache was newly created.
pub fn open(conn: &SqliteConnection, name: &str) -> Result<bool, CacheError> {
    ensure_names_table(conn)?;

    let exists: bool = conn
        .raw_connection()
        .query_row(
            "SELECT COUNT(*) > 0 FROM _cache_names WHERE name = ?1",
            [name],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if exists {
        return Ok(false);
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    conn.raw_connection()
        .execute(
            "INSERT OR IGNORE INTO _cache_names (name, created_at) VALUES (?1, ?2)",
            rusqlite::params![name, now as i64],
        )
        .map_err(|e| CacheError::Storage(elidex_storage_core::StorageError::from(e)))?;

    Ok(true)
}

/// Check if a named cache exists.
pub fn has(conn: &SqliteConnection, name: &str) -> Result<bool, CacheError> {
    ensure_names_table(conn)?;

    let exists: bool = conn
        .raw_connection()
        .query_row(
            "SELECT COUNT(*) > 0 FROM _cache_names WHERE name = ?1",
            [name],
            |row| row.get(0),
        )
        .unwrap_or(false);

    Ok(exists)
}

/// Delete a named cache and all its entries.
pub fn delete(conn: &SqliteConnection, name: &str) -> Result<bool, CacheError> {
    ensure_names_table(conn)?;

    // Drop the cache data table
    let safe_name = sanitize_cache_name(name);
    let table = format!("cache_{safe_name}");
    conn.raw_connection()
        .execute_batch(&format!("DROP TABLE IF EXISTS [{table}]"))
        .map_err(|e| CacheError::Storage(elidex_storage_core::StorageError::from(e)))?;

    // Remove from registry
    let deleted: usize = conn
        .raw_connection()
        .execute("DELETE FROM _cache_names WHERE name = ?1", [name])
        .map_err(|e| CacheError::Storage(elidex_storage_core::StorageError::from(e)))?;

    Ok(deleted > 0)
}

/// List all cache names, in creation order.
pub fn keys(conn: &SqliteConnection) -> Result<Vec<String>, CacheError> {
    ensure_names_table(conn)?;

    let mut stmt = conn
        .raw_connection()
        .prepare("SELECT name FROM _cache_names ORDER BY created_at ASC")
        .map_err(|e| CacheError::Storage(elidex_storage_core::StorageError::from(e)))?;

    let names: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| CacheError::Storage(elidex_storage_core::StorageError::from(e)))?
        .collect::<Result<_, _>>()
        .map_err(|e| CacheError::Storage(elidex_storage_core::StorageError::from(e)))?;

    Ok(names)
}

/// Sanitize cache name for SQL table name (same as store.rs).
fn sanitize_cache_name(name: &str) -> String {
    name.bytes()
        .fold(String::with_capacity(name.len() * 2), |mut acc, b| {
            use std::fmt::Write;
            let _ = write!(acc, "{b:02x}");
            acc
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> SqliteConnection {
        SqliteConnection::open_in_memory().unwrap()
    }

    #[test]
    fn open_creates_cache() {
        let conn = setup();
        let created = open(&conn, "v1").unwrap();
        assert!(created);

        let created2 = open(&conn, "v1").unwrap();
        assert!(!created2); // already exists
    }

    #[test]
    fn has_checks_existence() {
        let conn = setup();
        assert!(!has(&conn, "v1").unwrap());
        open(&conn, "v1").unwrap();
        assert!(has(&conn, "v1").unwrap());
    }

    #[test]
    fn delete_removes_cache() {
        let conn = setup();
        open(&conn, "v1").unwrap();
        assert!(has(&conn, "v1").unwrap());

        let deleted = delete(&conn, "v1").unwrap();
        assert!(deleted);
        assert!(!has(&conn, "v1").unwrap());
    }

    #[test]
    fn delete_nonexistent() {
        let conn = setup();
        let deleted = delete(&conn, "nope").unwrap();
        assert!(!deleted);
    }

    #[test]
    fn keys_lists_caches_in_order() {
        let conn = setup();
        open(&conn, "first").unwrap();
        open(&conn, "second").unwrap();
        open(&conn, "third").unwrap();

        let names = keys(&conn).unwrap();
        assert_eq!(names, vec!["first", "second", "third"]);
    }

    #[test]
    fn keys_empty() {
        let conn = setup();
        let names = keys(&conn).unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn open_and_delete_with_data() {
        let conn = setup();
        open(&conn, "data-cache").unwrap();

        // Put some data
        let entry = crate::entry::CachedEntry {
            request_url: "https://example.com/".into(),
            request_method: "GET".into(),
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![],
            response_body: b"hello".to_vec(),
            vary_headers: vec![],
            is_opaque: false,
        };
        crate::store::put(&conn, "data-cache", &entry).unwrap();

        // Delete cache (should drop table)
        delete(&conn, "data-cache").unwrap();

        // Re-open and verify empty
        open(&conn, "data-cache").unwrap();
        let entries = crate::store::keys(&conn, "data-cache").unwrap();
        assert!(entries.is_empty());
    }
}
