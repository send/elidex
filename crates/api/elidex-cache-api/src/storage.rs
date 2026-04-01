//! CacheStorage operations (WHATWG Cache API §2.2).
//!
//! Manages named caches within an origin's storage.
//! The `_cache_names` registry is schema management (like IDB meta-tables)
//! and uses `raw_connection()` directly, consistent with the IDB pattern.

use elidex_storage_core::{SqliteConnection, StorageConnection};

use crate::error::CacheError;

/// Schema migration for the cache names registry.
const CACHE_NAMES_SCHEMA: &str =
    "CREATE TABLE IF NOT EXISTS _cache_names (name TEXT PRIMARY KEY, created_at INTEGER NOT NULL)";

/// Helper: convert rusqlite error to CacheError.
fn sql_err(e: rusqlite::Error) -> CacheError {
    CacheError::Storage(elidex_storage_core::StorageError::from(e))
}

/// Ensure the cache names table exists.
fn ensure_names_table(conn: &SqliteConnection) -> Result<(), CacheError> {
    conn.raw_connection()
        .execute_batch(CACHE_NAMES_SCHEMA)
        .map_err(sql_err)
}

/// Open (or create) a named cache.
///
/// If the cache doesn't exist, it is created and registered.
/// Returns `true` if the cache was newly created.
pub fn open(conn: &SqliteConnection, name: &str) -> Result<bool, CacheError> {
    ensure_names_table(conn)?;

    let raw = conn.raw_connection();
    let exists: bool = raw
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

    raw.execute(
        "INSERT OR IGNORE INTO _cache_names (name, created_at) VALUES (?1, ?2)",
        rusqlite::params![name, now.cast_signed()],
    )
    .map_err(sql_err)?;

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

    let raw = conn.raw_connection();
    let safe_name = elidex_storage_core::sanitize_sql_name(name);
    let table = format!("cache_{safe_name}");
    raw.execute_batch(&format!("DROP TABLE IF EXISTS [{table}]"))
        .map_err(sql_err)?;

    let deleted = raw
        .execute("DELETE FROM _cache_names WHERE name = ?1", [name])
        .map_err(sql_err)?;
    Ok(deleted > 0)
}

/// List all cache names, in creation order.
pub fn keys(conn: &SqliteConnection) -> Result<Vec<String>, CacheError> {
    ensure_names_table(conn)?;
    let mut stmt = conn
        .raw_connection()
        .prepare("SELECT name FROM _cache_names ORDER BY created_at ASC")
        .map_err(sql_err)?;
    let names: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(sql_err)?
        .collect::<Result<_, _>>()
        .map_err(sql_err)?;
    Ok(names)
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
        assert!(open(&conn, "v1").unwrap());
        assert!(!open(&conn, "v1").unwrap());
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
        assert!(delete(&conn, "v1").unwrap());
        assert!(!has(&conn, "v1").unwrap());
    }

    #[test]
    fn delete_nonexistent() {
        let conn = setup();
        assert!(!delete(&conn, "nope").unwrap());
    }

    #[test]
    fn keys_lists_caches_in_order() {
        let conn = setup();
        open(&conn, "first").unwrap();
        open(&conn, "second").unwrap();
        open(&conn, "third").unwrap();
        assert_eq!(keys(&conn).unwrap(), vec!["first", "second", "third"]);
    }

    #[test]
    fn keys_empty() {
        let conn = setup();
        assert!(keys(&conn).unwrap().is_empty());
    }

    #[test]
    fn open_and_delete_with_data() {
        let conn = setup();
        open(&conn, "data-cache").unwrap();

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

        delete(&conn, "data-cache").unwrap();
        open(&conn, "data-cache").unwrap();
        assert!(crate::store::keys(&conn, "data-cache").unwrap().is_empty());
    }
}
