use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::backend::{OpenOptions, StorageBackend};
use crate::error::StorageError;
use crate::quota::QuotaManager;
use crate::sqlite::{SqliteBackend, SqliteConnection};

/// Storage types that an origin can use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StorageType {
    IndexedDb,
    CacheApi,
    KeyValue,
}

impl StorageType {
    fn filename(&self) -> &str {
        match self {
            StorageType::IndexedDb => "idb.sqlite",
            StorageType::CacheApi => "cache.sqlite",
            StorageType::KeyValue => "kv.sqlite",
        }
    }
}

/// Origin key for storage isolation.
///
/// Typed struct with normalized fields (scheme/host/port) to eliminate
/// format ambiguity. All constructors produce the same canonical form,
/// ensuring correct `Eq`/`Hash` behavior in `HashMap` lookups.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OriginKey {
    scheme: String,
    host: String,
    port: u16,
}

impl OriginKey {
    /// Create from URL components. Normalizes scheme and host to lowercase.
    pub fn from_parts(scheme: &str, host: &str, port: u16) -> Self {
        Self {
            scheme: scheme.to_ascii_lowercase(),
            host: host.to_ascii_lowercase(),
            port,
        }
    }

    /// Create from a `url::Url`.
    pub fn from_url(url: &url::Url) -> Option<Self> {
        let scheme = url.scheme();
        let host = url.host_str()?;
        let port = url.port_or_known_default()?;
        Some(Self::from_parts(scheme, host, port))
    }

    /// Create from a `url::Origin` (Tuple variant only; opaque origins return `None`).
    pub fn from_origin(origin: &url::Origin) -> Option<Self> {
        match origin {
            url::Origin::Tuple(scheme, host, port) => {
                Some(Self::from_parts(scheme, &host.to_string(), *port))
            }
            url::Origin::Opaque(_) => None,
        }
    }

    /// Filesystem-safe directory name for this origin.
    ///
    /// Format: `"{scheme}_{host}_{port}"`, hex-encoded via `sanitize_sql_name`
    /// to avoid filesystem-unsafe characters (e.g., ':' in IPv6 on Windows).
    pub fn dir_name(&self) -> String {
        let raw = format!("{}_{}_{}", self.scheme, self.host, self.port);
        crate::util::sanitize_sql_name(&raw)
    }

    /// Scheme component (lowercase).
    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    /// Host component (lowercase).
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Port component.
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl std::fmt::Display for OriginKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}://{}:{}", self.scheme, self.host, self.port)
    }
}

/// Manages per-origin storage connections (Ch.22 OriginStorageManager).
///
/// Lazily opens databases on first access. Each origin gets a separate
/// directory under `profile_dir/origins/`, with one SQLite file per storage type.
///
/// Uses per-connection locking (`Arc<Mutex<SqliteConnection>>`) so that
/// operations on unrelated origins do not serialize against each other.
/// The global `connections` mutex is held only briefly for map lookups/inserts.
/// Connection map type alias to reduce type complexity.
type ConnectionMap = HashMap<(OriginKey, StorageType), Arc<Mutex<SqliteConnection>>>;

pub struct OriginStorageManager {
    profile_dir: PathBuf,
    backend: SqliteBackend,
    connections: Mutex<ConnectionMap>,
    pub quota_manager: QuotaManager,
}

impl OriginStorageManager {
    pub fn new(profile_dir: PathBuf) -> Self {
        Self {
            profile_dir,
            backend: SqliteBackend::new(),
            connections: Mutex::new(HashMap::new()),
            quota_manager: QuotaManager::new(),
        }
    }

    /// Get or open a connection for the given origin and storage type.
    ///
    /// Uses double-check pattern: checks under lock, drops lock for I/O if
    /// needed, then re-acquires to insert (with a second check for races).
    fn ensure_connection(
        &self,
        origin: &OriginKey,
        storage_type: StorageType,
    ) -> Result<Arc<Mutex<SqliteConnection>>, StorageError> {
        let key = (origin.clone(), storage_type);

        // Fast path: connection already exists.
        {
            let conns = self.connections.lock().unwrap();
            if let Some(conn) = conns.get(&key) {
                return Ok(Arc::clone(conn));
            }
        }

        // Slow path: open outside the lock (filesystem I/O).
        let path = self.db_path(origin, storage_type);
        let new_conn = self.backend.open(&path, OpenOptions::default())?;

        // Re-acquire and double-check (another thread may have inserted).
        let mut conns = self.connections.lock().unwrap();
        let entry = conns
            .entry(key)
            .or_insert_with(|| Arc::new(Mutex::new(new_conn)));
        Ok(Arc::clone(entry))
    }

    /// Get or open a connection for the given origin and storage type.
    pub fn connection(
        &self,
        origin: &OriginKey,
        storage_type: StorageType,
    ) -> Result<(), StorageError> {
        self.ensure_connection(origin, storage_type)?;
        Ok(())
    }

    /// Execute an operation on a connection, opening it if necessary.
    ///
    /// The global connections map is locked only briefly to clone the `Arc`.
    /// The per-connection mutex is held for the duration of `f`.
    pub fn with_connection<F, T>(
        &self,
        origin: &OriginKey,
        storage_type: StorageType,
        f: F,
    ) -> Result<T, StorageError>
    where
        F: FnOnce(&SqliteConnection) -> Result<T, StorageError>,
    {
        let conn_arc = self.ensure_connection(origin, storage_type)?;
        let conn = conn_arc.lock().unwrap();
        f(&conn)
    }

    /// Run migrations on a connection.
    pub fn migrate(
        &self,
        origin: &OriginKey,
        storage_type: StorageType,
        migrations: &[crate::backend::Migration],
    ) -> Result<(), StorageError> {
        let conn_arc = self.ensure_connection(origin, storage_type)?;
        let conn = conn_arc.lock().unwrap();
        self.backend.migrate(&conn, migrations)
    }

    /// Compute the path for a database file.
    fn db_path(&self, origin: &OriginKey, storage_type: StorageType) -> PathBuf {
        self.profile_dir
            .join("origins")
            .join(origin.dir_name())
            .join(storage_type.filename())
    }

    /// Get the origin directory path (for quota estimation).
    pub fn origin_dir(&self, origin: &OriginKey) -> PathBuf {
        self.profile_dir.join("origins").join(origin.dir_name())
    }

    /// Close all connections for an origin (e.g., on eviction).
    pub fn close_origin(&self, origin: &OriginKey) {
        let mut conns = self.connections.lock().unwrap();
        conns.retain(|(k, _), _| k != origin);
    }

    /// Close all connections.
    pub fn close_all(&self) {
        let mut conns = self.connections.lock().unwrap();
        conns.clear();
    }
}

impl std::fmt::Debug for OriginStorageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self.connections.lock().unwrap().len();
        f.debug_struct("OriginStorageManager")
            .field("profile_dir", &self.profile_dir)
            .field("open_connections", &count)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_key_from_parts() {
        let key = OriginKey::from_parts("https", "example.com", 443);
        assert_eq!(key.scheme(), "https");
        assert_eq!(key.host(), "example.com");
        assert_eq!(key.port(), 443);
    }

    #[test]
    fn origin_key_from_parts_normalizes() {
        let key = OriginKey::from_parts("HTTPS", "Example.COM", 443);
        assert_eq!(key.scheme(), "https");
        assert_eq!(key.host(), "example.com");
    }

    #[test]
    fn origin_key_from_url() {
        let url = url::Url::parse("https://example.com:443/path").unwrap();
        let key = OriginKey::from_url(&url).unwrap();
        assert_eq!(key.scheme(), "https");
        assert_eq!(key.host(), "example.com");
        assert_eq!(key.port(), 443);
    }

    #[test]
    fn origin_key_from_origin() {
        let url = url::Url::parse("https://example.com/page").unwrap();
        let origin = url.origin();
        let key = OriginKey::from_origin(&origin).unwrap();
        assert_eq!(key.scheme(), "https");
        assert_eq!(key.host(), "example.com");
        assert_eq!(key.port(), 443);
    }

    #[test]
    fn origin_key_from_url_and_from_origin_are_equal() {
        let url = url::Url::parse("https://example.com:443/path?q=1").unwrap();
        let from_url = OriginKey::from_url(&url).unwrap();
        let from_origin = OriginKey::from_origin(&url.origin()).unwrap();
        assert_eq!(from_url, from_origin);
    }

    #[test]
    fn origin_key_display() {
        let key = OriginKey::from_parts("https", "example.com", 443);
        assert_eq!(key.to_string(), "https://example.com:443");
    }

    #[test]
    fn origin_key_dir_name() {
        let key = OriginKey::from_parts("https", "example.com", 443);
        let dir = key.dir_name();
        let expected = crate::util::sanitize_sql_name("https_example.com_443");
        assert_eq!(dir, expected);
    }

    #[test]
    fn origin_key_opaque_returns_none() {
        let origin = url::Origin::new_opaque();
        assert!(OriginKey::from_origin(&origin).is_none());
    }

    #[test]
    fn storage_type_filenames() {
        assert_eq!(StorageType::IndexedDb.filename(), "idb.sqlite");
        assert_eq!(StorageType::CacheApi.filename(), "cache.sqlite");
        assert_eq!(StorageType::KeyValue.filename(), "kv.sqlite");
    }

    #[test]
    fn db_path_construction() {
        let mgr = OriginStorageManager::new(PathBuf::from("/tmp/elidex-test"));
        let origin = OriginKey::from_parts("https", "example.com", 443);
        let path = mgr.db_path(&origin, StorageType::CacheApi);
        let expected_dir = origin.dir_name();
        assert_eq!(
            path,
            PathBuf::from(format!(
                "/tmp/elidex-test/origins/{expected_dir}/cache.sqlite"
            ))
        );
    }

    #[test]
    fn open_and_use_connection() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = OriginStorageManager::new(dir.path().to_path_buf());
        let origin = OriginKey::from_parts("https", "test.local", 443);

        mgr.with_connection(&origin, StorageType::KeyValue, |conn| {
            use crate::backend::StorageConnection;
            conn.raw_connection()
                .execute_batch(
                    "CREATE TABLE IF NOT EXISTS [test] (key BLOB PRIMARY KEY, value BLOB NOT NULL)",
                )
                .map_err(StorageError::from)?;

            conn.execute(&crate::backend::StorageOp::Put {
                table: "test",
                key: b"k1",
                value: b"v1",
            })?;

            match conn.execute(&crate::backend::StorageOp::Get {
                table: "test",
                key: b"k1",
            })? {
                crate::backend::StorageResult::Row(v) => assert_eq!(v, b"v1"),
                other => panic!("expected Row, got {other:?}"),
            }
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn close_origin() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = OriginStorageManager::new(dir.path().to_path_buf());
        let origin = OriginKey::from_parts("https", "close.test", 443);

        mgr.connection(&origin, StorageType::KeyValue).unwrap();
        assert_eq!(mgr.connections.lock().unwrap().len(), 1);

        mgr.close_origin(&origin);
        assert_eq!(mgr.connections.lock().unwrap().len(), 0);
    }
}
