//! `IndexedDB` database open/delete/upgrade protocol.
//!
//! Implements the W3C `IndexedDB` 3.0 §2.4 open/upgrade algorithm:
//! - DB not found → create + `upgradeneeded`
//! - version > current → `upgradeneeded` (versionchange transaction)
//! - version < current → `VersionError`
//! - version == current → success

use crate::backend::{BackendError, IdbBackend};

/// Result of an `open_database` call.
#[derive(Debug)]
pub enum IdbOpenResult {
    /// Database opened successfully at the requested version.
    Success(IdbDatabaseHandle),
    /// An upgrade is needed. The caller must run the `upgradeneeded` callback
    /// inside a versionchange transaction, then call `finish_upgrade()`.
    UpgradeNeeded {
        handle: IdbDatabaseHandle,
        old_version: u64,
        new_version: u64,
    },
}

/// A handle to an opened `IndexedDB` database.
#[derive(Debug)]
pub struct IdbDatabaseHandle {
    db_name: String,
    version: u64,
}

impl IdbDatabaseHandle {
    /// Returns the database name.
    pub fn name(&self) -> &str {
        &self.db_name
    }

    /// Returns the current version.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// List object store names for this database.
    pub fn object_store_names(&self, backend: &IdbBackend) -> Result<Vec<String>, BackendError> {
        backend.list_store_names(&self.db_name)
    }
}

/// Open a database with the W3C `IndexedDB` open algorithm.
///
/// - If `version` is `None`, opens at the current version (or 1 if new).
/// - If `version` is `Some(0)`, returns `DataError`.
/// - If `version` is `Some(v)` where `v < current`, returns `VersionError`.
pub fn open_database(
    backend: &IdbBackend,
    db_name: &str,
    version: Option<u64>,
) -> Result<IdbOpenResult, BackendError> {
    // Spec: version 0 is not allowed
    if version == Some(0) {
        return Err(BackendError::DataError("version must not be 0".into()));
    }

    let current_version = backend.get_version(db_name)?;

    match current_version {
        None => {
            // Database doesn't exist — create it
            let new_version = version.unwrap_or(1);
            backend.set_version(db_name, new_version)?;
            Ok(IdbOpenResult::UpgradeNeeded {
                handle: IdbDatabaseHandle {
                    db_name: db_name.to_owned(),
                    version: new_version,
                },
                old_version: 0,
                new_version,
            })
        }
        Some(current) => {
            let requested = version.unwrap_or(current);

            match requested.cmp(&current) {
                std::cmp::Ordering::Less => Err(BackendError::VersionError(format!(
                    "requested version {requested} < current version {current}"
                ))),
                std::cmp::Ordering::Greater => {
                    backend.set_version(db_name, requested)?;
                    Ok(IdbOpenResult::UpgradeNeeded {
                        handle: IdbDatabaseHandle {
                            db_name: db_name.to_owned(),
                            version: requested,
                        },
                        old_version: current,
                        new_version: requested,
                    })
                }
                std::cmp::Ordering::Equal => Ok(IdbOpenResult::Success(IdbDatabaseHandle {
                    db_name: db_name.to_owned(),
                    version: current,
                })),
            }
        }
    }
}

/// Delete a database entirely.
///
/// Returns the old version (0 if the database didn't exist).
pub fn delete_database(backend: &IdbBackend, db_name: &str) -> Result<u64, BackendError> {
    let old_version = backend.get_version(db_name)?.unwrap_or(0);
    backend.delete_database(db_name)?;
    Ok(old_version)
}

/// Finish an upgrade by committing the version. Called after the `upgradeneeded`
/// callback completes successfully. If the upgrade was aborted, call
/// `abort_upgrade` instead.
pub fn finish_upgrade(
    backend: &IdbBackend,
    handle: &IdbDatabaseHandle,
) -> Result<(), BackendError> {
    // Version was already set in open_database. This is a confirmation point
    // that the upgrade transaction committed successfully.
    // In the full implementation, this is where we'd verify the transaction state.
    let _ = backend.get_version(&handle.db_name)?;
    Ok(())
}

/// Abort an upgrade — revert to the old version.
pub fn abort_upgrade(
    backend: &IdbBackend,
    handle: &IdbDatabaseHandle,
    old_version: u64,
) -> Result<(), BackendError> {
    if old_version == 0 {
        // Database was newly created — delete it entirely
        backend.delete_database(&handle.db_name)?;
    } else {
        backend.set_version(&handle.db_name, old_version)?;
    }
    Ok(())
}

/// List all databases for this origin.
pub fn list_databases(backend: &IdbBackend) -> Result<Vec<(String, u64)>, BackendError> {
    let names = backend.list_database_names()?;
    let mut result = Vec::with_capacity(names.len());
    for name in names {
        let version = backend.get_version(&name)?.unwrap_or(0);
        result.push((name, version));
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_backend() -> IdbBackend {
        IdbBackend::open_in_memory().unwrap()
    }

    #[test]
    fn open_new_database_triggers_upgrade() {
        let b = mem_backend();
        let result = open_database(&b, "mydb", Some(1)).unwrap();
        match result {
            IdbOpenResult::UpgradeNeeded {
                handle,
                old_version,
                new_version,
            } => {
                assert_eq!(handle.name(), "mydb");
                assert_eq!(handle.version(), 1);
                assert_eq!(old_version, 0);
                assert_eq!(new_version, 1);
            }
            IdbOpenResult::Success(_) => panic!("expected UpgradeNeeded"),
        }
    }

    #[test]
    fn open_new_database_default_version() {
        let b = mem_backend();
        let result = open_database(&b, "mydb", None).unwrap();
        match result {
            IdbOpenResult::UpgradeNeeded { new_version, .. } => {
                assert_eq!(new_version, 1);
            }
            IdbOpenResult::Success(_) => panic!("expected UpgradeNeeded"),
        }
    }

    #[test]
    fn open_existing_same_version() {
        let b = mem_backend();
        b.set_version("mydb", 3).unwrap();
        let result = open_database(&b, "mydb", Some(3)).unwrap();
        match result {
            IdbOpenResult::Success(handle) => {
                assert_eq!(handle.version(), 3);
            }
            IdbOpenResult::UpgradeNeeded { .. } => panic!("expected Success"),
        }
    }

    #[test]
    fn open_existing_no_version_returns_current() {
        let b = mem_backend();
        b.set_version("mydb", 5).unwrap();
        let result = open_database(&b, "mydb", None).unwrap();
        match result {
            IdbOpenResult::Success(handle) => {
                assert_eq!(handle.version(), 5);
            }
            IdbOpenResult::UpgradeNeeded { .. } => panic!("expected Success"),
        }
    }

    #[test]
    fn open_existing_higher_version_triggers_upgrade() {
        let b = mem_backend();
        b.set_version("mydb", 2).unwrap();
        let result = open_database(&b, "mydb", Some(5)).unwrap();
        match result {
            IdbOpenResult::UpgradeNeeded {
                old_version,
                new_version,
                ..
            } => {
                assert_eq!(old_version, 2);
                assert_eq!(new_version, 5);
            }
            IdbOpenResult::Success(_) => panic!("expected UpgradeNeeded"),
        }
    }

    #[test]
    fn open_existing_lower_version_is_error() {
        let b = mem_backend();
        b.set_version("mydb", 5).unwrap();
        let err = open_database(&b, "mydb", Some(3));
        assert!(matches!(err, Err(BackendError::VersionError(_))));
    }

    #[test]
    fn open_version_zero_is_error() {
        let b = mem_backend();
        let err = open_database(&b, "mydb", Some(0));
        assert!(matches!(err, Err(BackendError::DataError(_))));
    }

    #[test]
    fn delete_existing_database() {
        let b = mem_backend();
        b.set_version("mydb", 3).unwrap();
        b.create_object_store("mydb", "store1", None, false)
            .unwrap();

        let old_ver = delete_database(&b, "mydb").unwrap();
        assert_eq!(old_ver, 3);
        assert_eq!(b.get_version("mydb").unwrap(), None);
    }

    #[test]
    fn delete_nonexistent_database() {
        let b = mem_backend();
        let old_ver = delete_database(&b, "nope").unwrap();
        assert_eq!(old_ver, 0);
    }

    #[test]
    fn abort_upgrade_new_db_deletes_it() {
        let b = mem_backend();
        let result = open_database(&b, "mydb", Some(1)).unwrap();
        match result {
            IdbOpenResult::UpgradeNeeded { handle, .. } => {
                abort_upgrade(&b, &handle, 0).unwrap();
                assert_eq!(b.get_version("mydb").unwrap(), None);
            }
            IdbOpenResult::Success(_) => panic!("expected UpgradeNeeded"),
        }
    }

    #[test]
    fn abort_upgrade_existing_db_reverts_version() {
        let b = mem_backend();
        b.set_version("mydb", 2).unwrap();
        let result = open_database(&b, "mydb", Some(5)).unwrap();
        match result {
            IdbOpenResult::UpgradeNeeded { handle, .. } => {
                abort_upgrade(&b, &handle, 2).unwrap();
                assert_eq!(b.get_version("mydb").unwrap(), Some(2));
            }
            IdbOpenResult::Success(_) => panic!("expected UpgradeNeeded"),
        }
    }

    #[test]
    fn list_databases_returns_all() {
        let b = mem_backend();
        b.set_version("alpha", 1).unwrap();
        b.set_version("beta", 3).unwrap();

        let mut dbs = list_databases(&b).unwrap();
        dbs.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(dbs, vec![("alpha".into(), 1), ("beta".into(), 3)]);
    }

    #[test]
    fn handle_object_store_names() {
        let b = mem_backend();
        b.set_version("mydb", 1).unwrap();
        b.create_object_store("mydb", "users", None, false).unwrap();
        b.create_object_store("mydb", "posts", None, false).unwrap();

        let handle = IdbDatabaseHandle {
            db_name: "mydb".into(),
            version: 1,
        };
        let names = handle.object_store_names(&b).unwrap();
        assert_eq!(names, vec!["posts", "users"]);
    }
}
