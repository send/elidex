//! Individual cache store operations (WHATWG Cache API §2.3).

use elidex_storage_core::{
    CustomOp, SqliteConnection, StorageConnection, StorageError, StorageOp, StorageResult,
};

use crate::entry::{entry_matches, CachedEntry, MatchOptions};
use crate::error::CacheError;

/// Maximum cache name length (bytes). Prevents overly long SQL table names.
/// Checked against sanitized name (which can expand to ~2x via hex encoding).
const MAX_TABLE_NAME_LEN: usize = 450;

/// Validate that a cache name can produce a valid table name.
pub fn validate_cache_name(name: &str) -> Result<(), CacheError> {
    let safe = elidex_storage_core::sanitize_sql_name(name);
    let full = format!("cache_{safe}");
    if full.len() > MAX_TABLE_NAME_LEN {
        return Err(CacheError::Invalid(format!(
            "cache name too long ({} bytes sanitized, max {MAX_TABLE_NAME_LEN})",
            full.len()
        )));
    }
    Ok(())
}

/// Schema migration for a named cache's data table.
fn ensure_cache_table(conn: &SqliteConnection, cache_name: &str) -> Result<(), CacheError> {
    let table = table_name(cache_name)?;
    conn.raw_connection()
        .execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS [{table}] (\
                 key BLOB NOT NULL PRIMARY KEY,\
                 value BLOB NOT NULL\
             )"
        ))
        .map_err(|e| CacheError::Storage(StorageError::from(e)))?;
    Ok(())
}

/// Put a request/response pair into the cache.
///
/// If an entry with the same URL + method already exists, it is replaced.
/// Response body must be cloned before calling (spec: stream consumed once).
pub fn put(
    conn: &SqliteConnection,
    cache_name: &str,
    entry: &CachedEntry,
) -> Result<(), CacheError> {
    ensure_cache_table(conn, cache_name)?;
    let table = table_name(cache_name)?;
    let key = entry.storage_key();
    let value = entry.serialize();
    conn.execute(&StorageOp::Put {
        table: &table,
        key: &key,
        value: &value,
    })?;
    Ok(())
}

/// Match a single entry from the cache (WHATWG Cache API §2.3.1).
///
/// Returns the first matching entry, considering Vary headers and match options.
pub fn match_request(
    conn: &SqliteConnection,
    cache_name: &str,
    url: &str,
    method: &str,
    request_headers: &[(String, String)],
    options: &MatchOptions,
) -> Result<Option<CachedEntry>, CacheError> {
    let entries = match_all(conn, cache_name, url, method, request_headers, options)?;
    Ok(entries.into_iter().next())
}

/// Match all entries from the cache.
pub fn match_all(
    conn: &SqliteConnection,
    cache_name: &str,
    url: &str,
    method: &str,
    request_headers: &[(String, String)],
    options: &MatchOptions,
) -> Result<Vec<CachedEntry>, CacheError> {
    let table = table_name(cache_name)?;

    // If ignoring search or vary, scan all entries and filter
    // Otherwise try exact key lookup first
    if options.ignore_search || options.ignore_vary || options.ignore_method {
        let all_entries = scan_all_entries(conn, &table)?;
        Ok(all_entries
            .into_iter()
            .filter(|e| entry_matches(e, url, method, request_headers, options))
            .collect())
    } else {
        let key = CachedEntry::make_key(url, method);
        let result = conn.execute(&StorageOp::Get {
            table: &table,
            key: &key,
        });
        match result {
            Ok(StorageResult::Row(data)) => {
                if let Some(entry) = CachedEntry::deserialize(&data) {
                    if entry_matches(&entry, url, method, request_headers, options) {
                        return Ok(vec![entry]);
                    }
                }
                Ok(vec![])
            }
            Err(e)
                if matches!(e.kind, elidex_storage_core::StorageErrorKind::NotFound)
                    || (matches!(e.kind, elidex_storage_core::StorageErrorKind::Sqlite)
                        && e.message.contains("no such table")) =>
            {
                // NotFound = key missing; "no such table" = cache never opened.
                Ok(vec![])
            }
            Err(e) => Err(CacheError::Storage(e)),
            _ => Ok(vec![]),
        }
    }
}

/// Delete a matching entry from the cache.
pub fn delete(
    conn: &SqliteConnection,
    cache_name: &str,
    url: &str,
    method: &str,
    request_headers: &[(String, String)],
    options: &MatchOptions,
) -> Result<bool, CacheError> {
    let table = table_name(cache_name)?;

    // If table doesn't exist, nothing to delete
    let exists: bool = conn
        .raw_connection()
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
            [&table],
            |row| row.get(0),
        )
        .map_err(|e| CacheError::Storage(elidex_storage_core::StorageError::from(e)))?;
    if !exists {
        return Ok(false);
    }

    if options.ignore_search || options.ignore_method {
        // Need to scan and delete matching entries
        let all_entries = scan_all_entries(conn, &table)?;
        let mut deleted = false;
        for entry in &all_entries {
            if entry_matches(entry, url, method, request_headers, options) {
                let key = entry.storage_key();
                conn.execute(&StorageOp::Delete {
                    table: &table,
                    key: &key,
                })?;
                deleted = true;
            }
        }
        Ok(deleted)
    } else {
        let key = CachedEntry::make_key(url, method);
        let result = conn.execute(&StorageOp::Delete {
            table: &table,
            key: &key,
        })?;
        match result {
            StorageResult::Count(n) => Ok(n > 0),
            _ => Ok(false),
        }
    }
}

/// List all request URLs (keys) in the cache.
pub fn keys(conn: &SqliteConnection, cache_name: &str) -> Result<Vec<CachedEntry>, CacheError> {
    let table = table_name(cache_name)?;
    scan_all_entries(conn, &table)
}

/// Fetch all entries and store them atomically (WHATWG Cache API §2.3.2).
///
/// If ANY fetch/store fails, no entries are added (all-or-nothing).
/// The caller is responsible for fetching — this function receives already-fetched entries.
pub fn add_all(
    conn: &SqliteConnection,
    cache_name: &str,
    entries: &[CachedEntry],
) -> Result<(), CacheError> {
    ensure_cache_table(conn, cache_name)?;

    let table = table_name(cache_name)?;
    conn.transaction(|txn| {
        for entry in entries {
            let key = entry.storage_key();
            let value = entry.serialize();
            txn.execute(&StorageOp::Put {
                table: &table,
                key: &key,
                value: &value,
            })?;
        }
        Ok(())
    })?;
    Ok(())
}

// -- Internal helpers --

fn table_name(cache_name: &str) -> Result<String, CacheError> {
    let safe = elidex_storage_core::sanitize_sql_name(cache_name);
    let name = format!("cache_{safe}");
    if name.len() > MAX_TABLE_NAME_LEN {
        return Err(CacheError::Invalid(format!(
            "cache name too long (table name {} bytes, max {MAX_TABLE_NAME_LEN})",
            name.len()
        )));
    }
    Ok(name)
}

/// Custom operation to scan all entries from a cache table.
struct ScanAllOp {
    table: String,
}

impl CustomOp for ScanAllOp {
    fn execute(&self, conn: &rusqlite::Connection) -> Result<StorageResult, StorageError> {
        let sql = format!("SELECT value FROM [{}] ORDER BY key", self.table);
        let mut stmt = conn.prepare(&sql)?;
        let rows: Vec<Vec<u8>> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<_, _>>()?;
        Ok(StorageResult::Rows(rows))
    }
}

fn scan_all_entries(conn: &SqliteConnection, table: &str) -> Result<Vec<CachedEntry>, CacheError> {
    // Check if table exists first
    let exists: bool = conn
        .raw_connection()
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |row| row.get(0),
        )
        .map_err(|e| CacheError::Storage(elidex_storage_core::StorageError::from(e)))?;

    if !exists {
        return Ok(vec![]);
    }

    let op = StorageOp::Custom(Box::new(ScanAllOp {
        table: table.to_owned(),
    }));
    match conn.execute(&op)? {
        StorageResult::Rows(rows) => Ok(rows
            .into_iter()
            .filter_map(|data| CachedEntry::deserialize(&data))
            .collect()),
        _ => Ok(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> SqliteConnection {
        SqliteConnection::open_in_memory().unwrap()
    }

    fn sample_entry(url: &str) -> CachedEntry {
        CachedEntry {
            request_url: url.to_owned(),
            request_method: "GET".into(),
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![("content-type".into(), "text/html".into())],
            response_body: b"<h1>Hello</h1>".to_vec(),
            vary_headers: vec![],
            is_opaque: false,
        }
    }

    #[test]
    fn put_and_match() {
        let conn = setup();
        let entry = sample_entry("https://example.com/");
        put(&conn, "v1", &entry).unwrap();

        let result = match_request(
            &conn,
            "v1",
            "https://example.com/",
            "GET",
            &[],
            &MatchOptions::default(),
        )
        .unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().response_status, 200);
    }

    #[test]
    fn match_not_found() {
        let conn = setup();
        let result = match_request(
            &conn,
            "v1",
            "https://example.com/missing",
            "GET",
            &[],
            &MatchOptions::default(),
        )
        .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn put_overwrites() {
        let conn = setup();
        let mut entry = sample_entry("https://example.com/");
        put(&conn, "v1", &entry).unwrap();

        entry.response_body = b"updated".to_vec();
        put(&conn, "v1", &entry).unwrap();

        let result = match_request(
            &conn,
            "v1",
            "https://example.com/",
            "GET",
            &[],
            &MatchOptions::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(result.response_body, b"updated");
    }

    #[test]
    fn delete_entry() {
        let conn = setup();
        put(&conn, "v1", &sample_entry("https://example.com/")).unwrap();

        let deleted = delete(
            &conn,
            "v1",
            "https://example.com/",
            "GET",
            &[],
            &MatchOptions::default(),
        )
        .unwrap();
        assert!(deleted);

        let result = match_request(
            &conn,
            "v1",
            "https://example.com/",
            "GET",
            &[],
            &MatchOptions::default(),
        )
        .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn delete_nonexistent() {
        let conn = setup();
        let deleted = delete(
            &conn,
            "v1",
            "https://example.com/nope",
            "GET",
            &[],
            &MatchOptions::default(),
        )
        .unwrap();
        assert!(!deleted);
    }

    #[test]
    fn keys_lists_entries() {
        let conn = setup();
        put(&conn, "v1", &sample_entry("https://example.com/a")).unwrap();
        put(&conn, "v1", &sample_entry("https://example.com/b")).unwrap();

        let entries = keys(&conn, "v1").unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn add_all_atomicity() {
        let conn = setup();
        let entries = vec![
            sample_entry("https://example.com/1"),
            sample_entry("https://example.com/2"),
            sample_entry("https://example.com/3"),
        ];
        add_all(&conn, "v1", &entries).unwrap();

        let stored = keys(&conn, "v1").unwrap();
        assert_eq!(stored.len(), 3);
    }

    #[test]
    fn separate_caches_are_isolated() {
        let conn = setup();
        put(&conn, "cache-a", &sample_entry("https://example.com/")).unwrap();
        put(&conn, "cache-b", &sample_entry("https://other.com/")).unwrap();

        let a = keys(&conn, "cache-a").unwrap();
        let b = keys(&conn, "cache-b").unwrap();
        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 1);
        assert_eq!(a[0].request_url, "https://example.com/");
        assert_eq!(b[0].request_url, "https://other.com/");
    }
}
