//! Browser-owned centralized database (`browser.sqlite`).
//!
//! Stores cookies, history, bookmarks, permissions, settings, and other
//! browser-wide data. Owned exclusively by the Browser Process.
//!
//! Design doc: Ch.22 §22.4.1.

pub mod bookmarks;
pub mod cookies;
pub mod history;

use std::path::Path;

use crate::backend::StorageConnection;
use crate::error::StorageError;
use crate::sqlite::SqliteConnection;
use crate::{OpenOptions, SqliteBackend, StorageBackend};

/// Schema version 1: all browser-owned tables.
const SCHEMA_V1: &str = "\
CREATE TABLE IF NOT EXISTS cookies (
    host TEXT NOT NULL,
    path TEXT NOT NULL,
    name TEXT NOT NULL,
    partition_key TEXT NOT NULL DEFAULT '',
    value TEXT NOT NULL,
    domain TEXT NOT NULL,
    host_only INTEGER NOT NULL DEFAULT 1,
    persistent INTEGER NOT NULL DEFAULT 0,
    expires INTEGER,
    secure INTEGER NOT NULL DEFAULT 0,
    httponly INTEGER NOT NULL DEFAULT 0,
    samesite TEXT NOT NULL DEFAULT 'Lax',
    creation_time INTEGER NOT NULL,
    last_access_time INTEGER NOT NULL,
    PRIMARY KEY (host, path, name, partition_key)
);

CREATE TABLE IF NOT EXISTS urls (
    id INTEGER PRIMARY KEY,
    url TEXT UNIQUE NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    visit_count INTEGER NOT NULL DEFAULT 0,
    typed_count INTEGER NOT NULL DEFAULT 0,
    frecency INTEGER NOT NULL DEFAULT 0,
    last_visit_time INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_urls_frecency ON urls(frecency DESC);

CREATE TABLE IF NOT EXISTS visits (
    id INTEGER PRIMARY KEY,
    url_id INTEGER REFERENCES urls(id) ON DELETE CASCADE,
    visit_time INTEGER NOT NULL,
    transition_type INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_visits_time ON visits(visit_time);

CREATE TABLE IF NOT EXISTS bookmarks (
    id INTEGER PRIMARY KEY,
    parent_id INTEGER REFERENCES bookmarks(id) ON DELETE CASCADE,
    title TEXT NOT NULL DEFAULT '',
    url TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    date_added INTEGER NOT NULL,
    is_folder INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS permissions (
    origin TEXT NOT NULL,
    permission_type TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'prompt',
    expiry INTEGER,
    PRIMARY KEY (origin, permission_type)
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS cert_overrides (
    host TEXT NOT NULL,
    port INTEGER NOT NULL,
    cert_hash TEXT NOT NULL,
    expiry INTEGER,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (host, port)
);

CREATE TABLE IF NOT EXISTS content_prefs (
    origin TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (origin, key)
);
";

/// Browser-owned centralized database.
///
/// Wraps a single SQLite connection at `{profile_dir}/browser.sqlite`.
/// Sub-stores borrow the connection for zero-cost domain-specific access.
pub struct BrowserDb {
    conn: SqliteConnection,
}

impl BrowserDb {
    /// Open (or create) the browser database and run schema migrations.
    pub fn open(profile_dir: &Path) -> Result<Self, StorageError> {
        let db_path = profile_dir.join("browser.sqlite");
        let backend = SqliteBackend::new();
        let conn = backend.open(&db_path, OpenOptions::default())?;

        // Run schema migration.
        conn.raw_connection()
            .execute_batch(SCHEMA_V1)
            .map_err(StorageError::from)?;

        Ok(Self { conn })
    }

    /// Cookie persistence sub-store.
    pub fn cookies(&self) -> cookies::CookieStore<'_> {
        cookies::CookieStore::new(self.conn.raw_connection())
    }

    /// History sub-store.
    pub fn history(&self) -> history::HistoryStore<'_> {
        history::HistoryStore::new(self.conn.raw_connection())
    }

    /// Bookmark sub-store.
    pub fn bookmarks(&self) -> bookmarks::BookmarkStore<'_> {
        bookmarks::BookmarkStore::new(self.conn.raw_connection())
    }

    /// Raw connection access (for permissions, settings, etc.).
    pub fn raw_connection(&self) -> &rusqlite::Connection {
        self.conn.raw_connection()
    }
}

/// Convert `SystemTime` to Unix timestamp (seconds).
pub fn system_time_to_unix(t: std::time::SystemTime) -> i64 {
    t.duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Convert Unix timestamp (seconds) to `SystemTime`. Returns `None` for negative values. Zero returns `UNIX_EPOCH`.
pub fn unix_to_system_time(ts: i64) -> Option<std::time::SystemTime> {
    if ts < 0 {
        return None;
    }
    #[allow(clippy::cast_sign_loss)]
    Some(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(ts as u64))
}

impl std::fmt::Debug for BrowserDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserDb").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_creates_tables() {
        let dir = tempfile::tempdir().unwrap();
        let db = BrowserDb::open(dir.path()).unwrap();
        let conn = db.raw_connection();

        // Verify all 8 tables exist.
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();

        assert!(tables.contains(&"cookies".to_string()));
        assert!(tables.contains(&"urls".to_string()));
        assert!(tables.contains(&"visits".to_string()));
        assert!(tables.contains(&"bookmarks".to_string()));
        assert!(tables.contains(&"permissions".to_string()));
        assert!(tables.contains(&"settings".to_string()));
        assert!(tables.contains(&"cert_overrides".to_string()));
        assert!(tables.contains(&"content_prefs".to_string()));
    }

    #[test]
    fn open_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let db1 = BrowserDb::open(dir.path()).unwrap();
        drop(db1);
        // Re-opening should not fail.
        let _db2 = BrowserDb::open(dir.path()).unwrap();
    }
}
