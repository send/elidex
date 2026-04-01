use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

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

/// Origin key for storage isolation (e.g., "https_example.com_443").
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OriginKey(pub String);

impl OriginKey {
    /// Create from URL components.
    pub fn from_parts(scheme: &str, host: &str, port: u16) -> Self {
        Self(format!("{scheme}_{host}_{port}"))
    }

    /// Create from a url::Url.
    pub fn from_url(url: &url::Url) -> Option<Self> {
        let scheme = url.scheme();
        let host = url.host_str()?;
        let port = url.port_or_known_default()?;
        Some(Self::from_parts(scheme, host, port))
    }

    /// Filesystem-safe directory name for this origin.
    ///
    /// Hex-encodes the origin string to avoid filesystem-unsafe characters
    /// (e.g., ':' in IPv6 addresses is illegal on Windows).
    pub fn dir_name(&self) -> String {
        crate::util::sanitize_sql_name(&self.0)
    }
}

/// Manages per-origin storage connections (Ch.22 OriginStorageManager).
///
/// Lazily opens databases on first access. Each origin gets a separate
/// directory under `profile_dir/origins/`, with one SQLite file per storage type.
pub struct OriginStorageManager {
    profile_dir: PathBuf,
    backend: SqliteBackend,
    connections: Mutex<HashMap<(OriginKey, StorageType), SqliteConnection>>,
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
    pub fn connection(
        &self,
        origin: &OriginKey,
        storage_type: StorageType,
    ) -> Result<(), StorageError> {
        let key = (origin.clone(), storage_type);
        let mut conns = self.connections.lock().unwrap();
        if conns.contains_key(&key) {
            return Ok(());
        }

        let path = self.db_path(origin, storage_type);
        let conn = self.backend.open(&path, OpenOptions::default())?;
        conns.insert(key, conn);
        Ok(())
    }

    /// Execute an operation on a connection, opening it if necessary.
    ///
    /// Note: the connections mutex is held for the duration of `f`.
    /// The closure must not call back into `OriginStorageManager` to avoid deadlock.
    /// TODO(M4-8.5): refactor to per-connection locking (Arc<Mutex<SqliteConnection>>)
    /// to avoid serializing unrelated origins.
    pub fn with_connection<F, T>(
        &self,
        origin: &OriginKey,
        storage_type: StorageType,
        f: F,
    ) -> Result<T, StorageError>
    where
        F: FnOnce(&SqliteConnection) -> Result<T, StorageError>,
    {
        self.connection(origin, storage_type)?;
        let key = (origin.clone(), storage_type);
        let conns = self.connections.lock().unwrap();
        let conn = conns.get(&key).unwrap();
        f(conn)
    }

    /// Run migrations on a connection.
    pub fn migrate(
        &self,
        origin: &OriginKey,
        storage_type: StorageType,
        migrations: &[crate::backend::Migration],
    ) -> Result<(), StorageError> {
        self.connection(origin, storage_type)?;
        let key = (origin.clone(), storage_type);
        let conns = self.connections.lock().unwrap();
        let conn = conns.get(&key).unwrap();
        self.backend.migrate(conn, migrations)
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
        assert_eq!(key.0, "https_example.com_443");
    }

    #[test]
    fn origin_key_from_url() {
        let url = url::Url::parse("https://example.com:443/path").unwrap();
        let key = OriginKey::from_url(&url).unwrap();
        assert_eq!(key.0, "https_example.com_443");
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
        // Origin key contains '.', triggering hex encoding with "x_" prefix.
        let expected_dir = crate::util::sanitize_sql_name("https_example.com_443");
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
