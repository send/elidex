//! Individual cache store operations (WHATWG Cache API §2.3).
//!
//! Uses a unified `caches` + `entries` schema (design doc §22.5.5)
//! instead of per-cache tables. All entries are stored in a single
//! `entries` table with a `cache_id` foreign key.

use elidex_storage_core::{SqliteConnection, StorageConnection, StorageError};

use crate::entry::{entry_matches, CachedEntry, MatchOptions};
use crate::error::CacheError;

/// Maximum cache name length (bytes).
const MAX_CACHE_NAME_LEN: usize = 512;

/// Validate cache name length.
pub fn validate_cache_name(name: &str) -> Result<(), CacheError> {
    if name.len() > MAX_CACHE_NAME_LEN {
        return Err(CacheError::Invalid(format!(
            "cache name too long ({} bytes, max {MAX_CACHE_NAME_LEN})",
            name.len()
        )));
    }
    Ok(())
}

/// Ensure the unified schema exists.
pub(crate) fn ensure_schema(conn: &SqliteConnection) -> Result<(), CacheError> {
    conn.raw_connection().execute_batch(
        "CREATE TABLE IF NOT EXISTS caches (
                id INTEGER PRIMARY KEY,
                name TEXT UNIQUE NOT NULL,
                created_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS entries (
                cache_id INTEGER NOT NULL REFERENCES caches(id) ON DELETE CASCADE,
                request_url TEXT NOT NULL,
                request_method TEXT NOT NULL DEFAULT 'GET',
                request_headers TEXT,
                vary_header TEXT,
                response_status INTEGER NOT NULL,
                response_status_text TEXT NOT NULL DEFAULT '',
                response_headers TEXT NOT NULL,
                response_url_list TEXT,
                response_type TEXT NOT NULL DEFAULT 'basic',
                body BLOB,
                body_size INTEGER NOT NULL DEFAULT 0,
                is_opaque INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (cache_id, request_url, request_method)
            );",
    )?;
    Ok(())
}

/// Compute the `table_name_for` helper (kept for backward-compat with storage.rs delete).
pub(crate) fn table_name_for(cache_name: &str) -> Result<String, CacheError> {
    validate_cache_name(cache_name)?;
    let safe = elidex_storage_core::sanitize_sql_name(cache_name);
    Ok(format!("cache_{safe}"))
}

/// Get or create a cache_id for the given name.
fn get_or_create_cache_id(conn: &rusqlite::Connection, name: &str) -> Result<i64, CacheError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    conn.execute(
        "INSERT OR IGNORE INTO caches (name, created_at) VALUES (?1, ?2)",
        rusqlite::params![name, i64::try_from(now).unwrap_or(i64::MAX)],
    )?;

    conn.query_row("SELECT id FROM caches WHERE name = ?1", [name], |row| {
        row.get(0)
    })
    .map_err(CacheError::from)
}

/// Get cache_id for a name, returning None if it doesn't exist.
fn get_cache_id(conn: &rusqlite::Connection, name: &str) -> Result<Option<i64>, CacheError> {
    match conn.query_row("SELECT id FROM caches WHERE name = ?1", [name], |row| {
        row.get::<_, i64>(0)
    }) {
        Ok(id) => Ok(Some(id)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(CacheError::from(e)),
    }
}

/// Serialize headers to JSON string for storage.
fn headers_to_json(headers: &[(String, String)]) -> String {
    serde_json::to_string(headers).unwrap_or_else(|_| "[]".into())
}

/// Deserialize headers from JSON string.
fn headers_from_json(s: &str) -> Vec<(String, String)> {
    serde_json::from_str(s).unwrap_or_default()
}

/// Serialize URL list to JSON string.
fn url_list_to_json(urls: &[String]) -> String {
    serde_json::to_string(urls).unwrap_or_else(|_| "[]".into())
}

/// Deserialize URL list from JSON string.
fn url_list_from_json(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

/// Insert or replace a single entry. Shared by `put()` and `add_all()`.
fn insert_entry(
    raw: &rusqlite::Connection,
    cache_id: i64,
    entry: &CachedEntry,
    now_secs: i64,
) -> Result<(), rusqlite::Error> {
    raw.execute(
        "INSERT OR REPLACE INTO entries \
         (cache_id, request_url, request_method, request_headers, vary_header, \
          response_status, response_status_text, response_headers, response_url_list, \
          response_type, body, body_size, is_opaque, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        rusqlite::params![
            cache_id,
            entry.request_url,
            entry.request_method,
            headers_to_json(&entry.request_headers),
            headers_to_json(&entry.vary_headers),
            entry.response_status,
            entry.response_status_text,
            headers_to_json(&entry.response_headers),
            url_list_to_json(&entry.response_url_list),
            entry.response_type.as_str(),
            entry.response_body,
            entry.response_body.len() as i64,
            i32::from(entry.is_opaque),
            now_secs,
        ],
    )?;
    Ok(())
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Put a request/response pair into the cache.
pub fn put(
    conn: &SqliteConnection,
    cache_name: &str,
    entry: &CachedEntry,
) -> Result<(), CacheError> {
    validate_cache_name(cache_name)?;
    ensure_schema(conn)?;
    let raw = conn.raw_connection();
    let cache_id = get_or_create_cache_id(raw, cache_name)?;
    insert_entry(raw, cache_id, entry, now_secs())?;
    Ok(())
}

/// Read a `CachedEntry` from a row.
fn entry_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CachedEntry> {
    Ok(CachedEntry {
        request_url: row.get(0)?,
        request_method: row.get(1)?,
        request_headers: headers_from_json(&row.get::<_, String>(2)?),
        response_status: row.get::<_, i32>(3)? as u16,
        response_status_text: row.get(4)?,
        response_headers: headers_from_json(&row.get::<_, String>(5)?),
        response_body: row.get(6)?,
        response_url_list: url_list_from_json(&row.get::<_, String>(7)?),
        response_type: crate::entry::ResponseType::from_str_lossy(&row.get::<_, String>(8)?),
        vary_headers: headers_from_json(&row.get::<_, String>(9)?),
        is_opaque: row.get::<_, i32>(10)? != 0,
    })
}

/// Match a single entry from the cache.
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
    ensure_schema(conn)?;
    let raw = conn.raw_connection();
    let Some(cache_id) = get_cache_id(raw, cache_name)? else {
        return Ok(vec![]);
    };

    let entries = if options.ignore_search || options.ignore_vary || options.ignore_method {
        // Scan all entries in this cache and filter in Rust.
        load_all_entries(raw, cache_id)?
    } else {
        // Exact lookup by URL + method.
        let mut stmt = raw.prepare(
            "SELECT request_url, request_method, request_headers, \
                 response_status, response_status_text, response_headers, \
                 body, response_url_list, response_type, vary_header, is_opaque \
                 FROM entries WHERE cache_id = ?1 AND request_url = ?2 AND request_method = ?3",
        )?;
        let rows: Vec<CachedEntry> = stmt
            .query_map(rusqlite::params![cache_id, url, method], entry_from_row)?
            .filter_map(Result::ok)
            .collect();
        rows
    };

    Ok(entries
        .into_iter()
        .filter(|e| entry_matches(e, url, method, request_headers, options))
        .collect())
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
    ensure_schema(conn)?;
    let raw = conn.raw_connection();
    let Some(cache_id) = get_cache_id(raw, cache_name)? else {
        return Ok(false);
    };

    if options.ignore_search || options.ignore_method {
        let all = load_all_entries(raw, cache_id)?;
        let mut deleted = false;
        for entry in &all {
            if entry_matches(entry, url, method, request_headers, options) {
                raw.execute(
                    "DELETE FROM entries WHERE cache_id = ?1 AND request_url = ?2 AND request_method = ?3",
                    rusqlite::params![cache_id, entry.request_url, entry.request_method],
                )
                ?;
                deleted = true;
            }
        }
        Ok(deleted)
    } else {
        let n = raw.execute(
            "DELETE FROM entries WHERE cache_id = ?1 AND request_url = ?2 AND request_method = ?3",
            rusqlite::params![cache_id, url, method],
        )?;
        Ok(n > 0)
    }
}

/// List all entries in the cache (for `cache.keys()`).
pub fn keys(conn: &SqliteConnection, cache_name: &str) -> Result<Vec<CachedEntry>, CacheError> {
    ensure_schema(conn)?;
    let raw = conn.raw_connection();
    let Some(cache_id) = get_cache_id(raw, cache_name)? else {
        return Ok(vec![]);
    };
    load_all_entries(raw, cache_id)
}

/// Atomically add all entries (WHATWG Cache API §2.3.2).
pub fn add_all(
    conn: &SqliteConnection,
    cache_name: &str,
    entries: &[CachedEntry],
) -> Result<(), CacheError> {
    validate_cache_name(cache_name)?;
    ensure_schema(conn)?;
    let raw = conn.raw_connection();
    let cache_id = get_or_create_cache_id(raw, cache_name)?;
    let now = now_secs();

    conn.transaction(|txn| {
        let raw_txn = txn.raw_connection();
        for entry in entries {
            insert_entry(raw_txn, cache_id, entry, now).map_err(StorageError::from)?;
        }
        Ok(())
    })?;
    Ok(())
}

/// Load all entries for a cache_id.
fn load_all_entries(
    raw: &rusqlite::Connection,
    cache_id: i64,
) -> Result<Vec<CachedEntry>, CacheError> {
    let mut stmt = raw.prepare(
        "SELECT request_url, request_method, request_headers, \
             response_status, response_status_text, response_headers, \
             body, response_url_list, response_type, vary_header, is_opaque \
             FROM entries WHERE cache_id = ?1 ORDER BY request_url",
    )?;
    let entries: Vec<CachedEntry> = stmt
        .query_map([cache_id], entry_from_row)?
        .filter_map(Result::ok)
        .collect();
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> SqliteConnection {
        let conn = SqliteConnection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
        conn
    }

    fn sample_entry(url: &str) -> CachedEntry {
        CachedEntry {
            request_url: url.to_owned(),
            request_method: "GET".into(),
            request_headers: vec![],
            response_status: 200,
            response_status_text: "OK".into(),
            response_headers: vec![("content-type".into(), "text/html".into())],
            response_body: b"<h1>Hello</h1>".to_vec(),
            response_url_list: vec![],
            response_type: crate::entry::ResponseType::Basic,
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
        let matched = result.unwrap();
        assert_eq!(matched.response_status, 200);
        assert_eq!(matched.response_type, crate::entry::ResponseType::Basic);
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

    #[test]
    fn request_headers_preserved() {
        let conn = setup();
        let mut entry = sample_entry("https://example.com/api");
        entry.request_headers = vec![("accept".into(), "application/json".into())];
        put(&conn, "v1", &entry).unwrap();

        let result = keys(&conn, "v1").unwrap();
        assert_eq!(result[0].request_headers.len(), 1);
        assert_eq!(result[0].request_headers[0].0, "accept");
    }

    #[test]
    fn response_url_list_preserved() {
        let conn = setup();
        let mut entry = sample_entry("https://example.com/final");
        entry.response_url_list = vec![
            "https://example.com/original".into(),
            "https://example.com/final".into(),
        ];
        put(&conn, "v1", &entry).unwrap();

        let result = match_request(
            &conn,
            "v1",
            "https://example.com/final",
            "GET",
            &[],
            &MatchOptions::default(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(result.response_url_list.len(), 2);
    }
}
