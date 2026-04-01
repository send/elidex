//! CacheStorage operations (WHATWG Cache API §2.2).
//!
//! Manages named caches via the unified `caches` table (design doc §22.5.5).

use elidex_storage_core::{SqliteConnection, StorageConnection};

use crate::error::CacheError;
use crate::store;

/// Open (or create) a named cache.
///
/// Returns `true` if the cache was newly created.
pub fn open(conn: &SqliteConnection, name: &str) -> Result<bool, CacheError> {
    store::validate_cache_name(name)?;
    store::ensure_schema(conn)?;

    let raw = conn.raw_connection();
    let exists: bool = raw.query_row(
        "SELECT COUNT(*) > 0 FROM caches WHERE name = ?1",
        [name],
        |row| row.get(0),
    )?;

    if exists {
        return Ok(false);
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    raw.execute(
        "INSERT OR IGNORE INTO caches (name, created_at) VALUES (?1, ?2)",
        rusqlite::params![name, i64::try_from(now).unwrap_or(i64::MAX)],
    )?;
    Ok(true)
}

/// Check if a named cache exists.
pub fn has(conn: &SqliteConnection, name: &str) -> Result<bool, CacheError> {
    store::ensure_schema(conn)?;
    let exists: bool = conn.raw_connection().query_row(
        "SELECT COUNT(*) > 0 FROM caches WHERE name = ?1",
        [name],
        |row| row.get(0),
    )?;
    Ok(exists)
}

/// Delete a named cache and all its entries (CASCADE).
pub fn delete(conn: &SqliteConnection, name: &str) -> Result<bool, CacheError> {
    store::ensure_schema(conn)?;
    let raw = conn.raw_connection();

    // Also drop legacy per-cache table if it exists (migration cleanup).
    if let Ok(table) = store::table_name_for(name) {
        let _ = raw.execute_batch(&format!("DROP TABLE IF EXISTS [{table}]"));
    }

    let deleted = raw.execute("DELETE FROM caches WHERE name = ?1", [name])?;
    Ok(deleted > 0)
}

/// List all cache names, in creation order.
pub fn keys(conn: &SqliteConnection) -> Result<Vec<String>, CacheError> {
    store::ensure_schema(conn)?;
    let mut stmt = conn
        .raw_connection()
        .prepare("SELECT name FROM caches ORDER BY created_at ASC, name ASC")?;
    let names: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> SqliteConnection {
        let conn = SqliteConnection::open_in_memory().unwrap();
        store::ensure_schema(&conn).unwrap();
        conn
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
            request_headers: vec![],
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![],
            response_body: b"hello".to_vec(),
            response_url_list: vec![],
            response_type: crate::entry::ResponseType::Basic,
            vary_headers: vec![],
            is_opaque: false,
        };
        crate::store::put(&conn, "data-cache", &entry).unwrap();

        delete(&conn, "data-cache").unwrap();
        open(&conn, "data-cache").unwrap();
        assert!(crate::store::keys(&conn, "data-cache").unwrap().is_empty());
    }
}
