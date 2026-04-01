//! Cookie persistence sub-store (design doc §22.4.2 CookiePersistence).

use crate::error::StorageError;

/// A persisted cookie row matching the `cookies` table schema.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct PersistedCookie {
    pub host: String,
    pub path: String,
    pub name: String,
    /// Empty string = first-party; non-empty = CHIPS partitioned origin (e.g. `"https://news.com"`).
    pub partition_key: String,
    pub value: String,
    pub domain: String,
    pub host_only: bool,
    pub persistent: bool,
    pub expires: Option<i64>,
    pub secure: bool,
    pub httponly: bool,
    pub samesite: String,
    pub creation_time: i64,
    pub last_access_time: i64,
}

/// Zero-cost borrow wrapper around the browser.sqlite connection.
pub struct CookieStore<'db> {
    conn: &'db rusqlite::Connection,
}

impl<'db> CookieStore<'db> {
    pub(crate) fn new(conn: &'db rusqlite::Connection) -> Self {
        Self { conn }
    }

    /// Load all persisted cookies.
    pub fn load_all(&self) -> Result<Vec<PersistedCookie>, StorageError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT host, path, name, partition_key, value, domain, \
                 host_only, persistent, expires, secure, httponly, samesite, \
                 creation_time, last_access_time FROM cookies",
            )
            .map_err(StorageError::from)?;

        let rows = stmt
            .query_map([], |row| {
                Ok(PersistedCookie {
                    host: row.get(0)?,
                    path: row.get(1)?,
                    name: row.get(2)?,
                    partition_key: row.get::<_, String>(3)?,
                    value: row.get(4)?,
                    domain: row.get(5)?,
                    host_only: row.get::<_, i32>(6)? != 0,
                    persistent: row.get::<_, i32>(7)? != 0,
                    expires: row.get(8)?,
                    secure: row.get::<_, i32>(9)? != 0,
                    httponly: row.get::<_, i32>(10)? != 0,
                    samesite: row.get(11)?,
                    creation_time: row.get(12)?,
                    last_access_time: row.get(13)?,
                })
            })
            .map_err(StorageError::from)?;

        let mut cookies = Vec::new();
        for row in rows {
            cookies.push(row.map_err(StorageError::from)?);
        }
        Ok(cookies)
    }

    /// Persist (upsert) a single cookie.
    pub fn persist(&self, cookie: &PersistedCookie) -> Result<(), StorageError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO cookies \
                 (host, path, name, partition_key, value, domain, host_only, persistent, \
                  expires, secure, httponly, samesite, creation_time, last_access_time) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                rusqlite::params![
                    cookie.host,
                    cookie.path,
                    cookie.name,
                    cookie.partition_key,
                    cookie.value,
                    cookie.domain,
                    i32::from(cookie.host_only),
                    i32::from(cookie.persistent),
                    cookie.expires,
                    i32::from(cookie.secure),
                    i32::from(cookie.httponly),
                    cookie.samesite,
                    cookie.creation_time,
                    cookie.last_access_time,
                ],
            )
            .map_err(StorageError::from)?;
        Ok(())
    }

    /// Delete a specific cookie by (host, path, name).
    /// Delete a specific cookie by (host, path, name, partition_key).
    pub fn delete(
        &self,
        host: &str,
        path: &str,
        name: &str,
        partition_key: &str,
    ) -> Result<(), StorageError> {
        self.conn
            .execute(
                "DELETE FROM cookies WHERE host = ?1 AND path = ?2 AND name = ?3 \
                 AND partition_key = ?4",
                rusqlite::params![host, path, name, partition_key],
            )
            .map_err(StorageError::from)?;
        Ok(())
    }

    /// Delete all expired cookies. Returns the number of deleted rows.
    pub fn delete_expired(&self, now_unix: i64) -> Result<usize, StorageError> {
        let count = self
            .conn
            .execute(
                "DELETE FROM cookies WHERE expires IS NOT NULL AND expires <= ?1",
                rusqlite::params![now_unix],
            )
            .map_err(StorageError::from)?;
        Ok(count)
    }

    /// Clear all cookies for a given host.
    pub fn clear_host(&self, host: &str) -> Result<(), StorageError> {
        self.conn
            .execute(
                "DELETE FROM cookies WHERE host = ?1",
                rusqlite::params![host],
            )
            .map_err(StorageError::from)?;
        Ok(())
    }

    /// Replace all cookies atomically (DELETE + bulk INSERT in a transaction).
    ///
    /// Handles cookie deletions correctly (unlike per-cookie upsert).
    pub fn sync_all(&self, cookies: &[PersistedCookie]) -> Result<(), StorageError> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(StorageError::from)?;
        tx.execute_batch("DELETE FROM cookies")
            .map_err(StorageError::from)?;
        for cookie in cookies {
            tx.execute(
                "INSERT INTO cookies \
                 (host, path, name, partition_key, value, domain, host_only, persistent, \
                  expires, secure, httponly, samesite, creation_time, last_access_time) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                rusqlite::params![
                    cookie.host,
                    cookie.path,
                    cookie.name,
                    cookie.partition_key,
                    cookie.value,
                    cookie.domain,
                    i32::from(cookie.host_only),
                    i32::from(cookie.persistent),
                    cookie.expires,
                    i32::from(cookie.secure),
                    i32::from(cookie.httponly),
                    cookie.samesite,
                    cookie.creation_time,
                    cookie.last_access_time,
                ],
            )
            .map_err(StorageError::from)?;
        }
        tx.commit().map_err(StorageError::from)?;
        Ok(())
    }
}

// Time conversion utilities are in the parent module (browser_db/mod.rs).
pub use super::{system_time_to_unix, unix_to_system_time};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser_db::BrowserDb;
    use std::time::SystemTime;

    fn test_db() -> (tempfile::TempDir, BrowserDb) {
        let dir = tempfile::tempdir().unwrap();
        let db = BrowserDb::open(dir.path()).unwrap();
        (dir, db)
    }

    fn sample_cookie() -> PersistedCookie {
        PersistedCookie {
            host: "example.com".into(),
            path: "/".into(),
            name: "sid".into(),
            partition_key: String::new(),
            value: "abc123".into(),
            domain: "example.com".into(),
            host_only: true,
            persistent: true,
            expires: Some(1_700_000_000),
            secure: true,
            httponly: true,
            samesite: "Lax".into(),
            creation_time: 1_690_000_000,
            last_access_time: 1_690_000_100,
        }
    }

    #[test]
    fn persist_and_load() {
        let (_dir, db) = test_db();
        let store = db.cookies();

        store.persist(&sample_cookie()).unwrap();
        let cookies = store.load_all().unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].name, "sid");
        assert_eq!(cookies[0].value, "abc123");
        assert!(cookies[0].host_only);
        assert!(cookies[0].secure);
    }

    #[test]
    fn persist_upsert() {
        let (_dir, db) = test_db();
        let store = db.cookies();

        let mut cookie = sample_cookie();
        store.persist(&cookie).unwrap();

        cookie.value = "updated".into();
        store.persist(&cookie).unwrap();

        let cookies = store.load_all().unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].value, "updated");
    }

    #[test]
    fn delete_specific() {
        let (_dir, db) = test_db();
        let store = db.cookies();

        store.persist(&sample_cookie()).unwrap();
        store.delete("example.com", "/", "sid", "").unwrap();

        let cookies = store.load_all().unwrap();
        assert!(cookies.is_empty());
    }

    #[test]
    fn delete_expired() {
        let (_dir, db) = test_db();
        let store = db.cookies();

        let mut c1 = sample_cookie();
        c1.expires = Some(100); // long expired
        store.persist(&c1).unwrap();

        let mut c2 = sample_cookie();
        c2.name = "fresh".into();
        c2.expires = Some(i64::MAX); // far future
        store.persist(&c2).unwrap();

        let deleted = store.delete_expired(1_000_000).unwrap();
        assert_eq!(deleted, 1);

        let cookies = store.load_all().unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].name, "fresh");
    }

    #[test]
    fn clear_host() {
        let (_dir, db) = test_db();
        let store = db.cookies();

        store.persist(&sample_cookie()).unwrap();

        let mut other = sample_cookie();
        other.host = "other.com".into();
        other.domain = "other.com".into();
        store.persist(&other).unwrap();

        store.clear_host("example.com").unwrap();

        let cookies = store.load_all().unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].host, "other.com");
    }

    #[test]
    fn partition_key_chips() {
        let (_dir, db) = test_db();
        let store = db.cookies();

        let mut c1 = sample_cookie();
        c1.partition_key = String::new(); // first-party
        store.persist(&c1).unwrap();

        let mut c2 = sample_cookie();
        c2.partition_key = "https://news.com".into(); // CHIPS partitioned
        c2.value = "partitioned".into();
        store.persist(&c2).unwrap();

        let cookies = store.load_all().unwrap();
        assert_eq!(cookies.len(), 2);
    }

    #[test]
    fn time_conversion_roundtrip() {
        let now = SystemTime::now();
        let unix = system_time_to_unix(now);
        let back = unix_to_system_time(unix).unwrap();
        let diff = now.duration_since(back).unwrap_or_default();
        assert!(diff.as_secs() < 2);
    }
}
