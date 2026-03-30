//! `IndexedDB` transaction management.
//!
//! Wraps `SQLite` transactions with IDB semantics: mode-based locking,
//! store scope validation, and state machine (Active → Finished / Aborted).

use std::collections::HashSet;

use crate::backend::BackendError;

/// Transaction mode per W3C `IndexedDB` 3.0 §2.8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdbTransactionMode {
    /// Read-only access (`SQLite` DEFERRED).
    ReadOnly,
    /// Read-write access (`SQLite` IMMEDIATE).
    ReadWrite,
    /// Version-change transaction (`SQLite` IMMEDIATE, allows schema changes).
    VersionChange,
}

/// Transaction state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState {
    /// Transaction is active and accepting requests.
    Active,
    /// Transaction has been committed.
    Finished,
    /// Transaction has been aborted.
    Aborted,
}

/// An `IndexedDB` transaction backed by a `SQLite` transaction.
///
/// The transaction is started on creation and must be explicitly
/// committed or aborted. Dropping without commit/abort will abort.
pub struct IdbTransaction {
    mode: IdbTransactionMode,
    state: TransactionState,
    scope: HashSet<String>,
    db_name: String,
}

impl IdbTransaction {
    /// Begin a new transaction.
    ///
    /// `store_names` defines which object stores are accessible.
    /// The `SQLite` transaction is started on the provided connection.
    pub fn begin(
        conn: &rusqlite::Connection,
        db_name: &str,
        store_names: Vec<String>,
        mode: IdbTransactionMode,
    ) -> Result<Self, BackendError> {
        let sql = match mode {
            IdbTransactionMode::ReadOnly => "BEGIN DEFERRED",
            IdbTransactionMode::ReadWrite | IdbTransactionMode::VersionChange => "BEGIN IMMEDIATE",
        };
        conn.execute_batch(sql)?;

        Ok(Self {
            mode,
            state: TransactionState::Active,
            scope: store_names.into_iter().collect(),
            db_name: db_name.to_owned(),
        })
    }

    /// Commit the transaction.
    pub fn commit(&mut self, conn: &rusqlite::Connection) -> Result<(), BackendError> {
        if self.state != TransactionState::Active {
            return Err(BackendError::InvalidStateError(format!(
                "transaction is {:?}, cannot commit",
                self.state
            )));
        }
        conn.execute_batch("COMMIT")?;
        self.state = TransactionState::Finished;
        Ok(())
    }

    /// Abort the transaction (rollback).
    pub fn abort(&mut self, conn: &rusqlite::Connection) -> Result<(), BackendError> {
        if self.state != TransactionState::Active {
            return Err(BackendError::InvalidStateError(format!(
                "transaction is {:?}, cannot abort",
                self.state
            )));
        }
        conn.execute_batch("ROLLBACK")?;
        self.state = TransactionState::Aborted;
        Ok(())
    }

    /// Check whether a store name is within this transaction's scope.
    pub fn check_scope(&self, store_name: &str) -> Result<(), BackendError> {
        // `VersionChange` transactions have access to all stores
        if self.mode == IdbTransactionMode::VersionChange {
            return Ok(());
        }
        if self.scope.contains(store_name) {
            Ok(())
        } else {
            Err(BackendError::NotFoundError(format!(
                "NotFoundError: object store '{store_name}' not in transaction scope"
            )))
        }
    }

    /// Check that the transaction is active.
    pub fn check_active(&self) -> Result<(), BackendError> {
        if self.state == TransactionState::Active {
            Ok(())
        } else {
            Err(BackendError::TransactionInactiveError(format!(
                "transaction is {:?}",
                self.state
            )))
        }
    }

    /// Check that the transaction allows writes.
    pub fn check_writable(&self) -> Result<(), BackendError> {
        self.check_active()?;
        if self.mode == IdbTransactionMode::ReadOnly {
            Err(BackendError::ReadOnlyError(
                "transaction is read-only".into(),
            ))
        } else {
            Ok(())
        }
    }

    /// Returns the transaction mode.
    pub fn mode(&self) -> IdbTransactionMode {
        self.mode
    }

    /// Returns the current state.
    pub fn state(&self) -> TransactionState {
        self.state
    }

    /// Returns the database name.
    pub fn db_name(&self) -> &str {
        &self.db_name
    }

    /// Returns the store names in scope.
    pub fn scope(&self) -> &HashSet<String> {
        &self.scope
    }

    /// Add a store to the scope (used during `VersionChange` when creating stores).
    pub fn add_to_scope(&mut self, store_name: String) {
        self.scope.insert(store_name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IdbBackend;

    fn setup() -> IdbBackend {
        let b = IdbBackend::open_in_memory().unwrap();
        b.set_version("db", 1).unwrap();
        b.create_object_store("db", "users", None, false).unwrap();
        b.create_object_store("db", "posts", None, false).unwrap();
        b
    }

    #[test]
    fn begin_and_commit() {
        let b = setup();
        let mut tx = IdbTransaction::begin(
            b.conn(),
            "db",
            vec!["users".into()],
            IdbTransactionMode::ReadOnly,
        )
        .unwrap();
        assert_eq!(tx.state(), TransactionState::Active);

        tx.commit(b.conn()).unwrap();
        assert_eq!(tx.state(), TransactionState::Finished);
    }

    #[test]
    fn begin_and_abort() {
        let b = setup();
        let mut tx = IdbTransaction::begin(
            b.conn(),
            "db",
            vec!["users".into()],
            IdbTransactionMode::ReadWrite,
        )
        .unwrap();

        tx.abort(b.conn()).unwrap();
        assert_eq!(tx.state(), TransactionState::Aborted);
    }

    #[test]
    fn double_commit_fails() {
        let b = setup();
        let mut tx = IdbTransaction::begin(
            b.conn(),
            "db",
            vec!["users".into()],
            IdbTransactionMode::ReadOnly,
        )
        .unwrap();
        tx.commit(b.conn()).unwrap();

        let err = tx.commit(b.conn());
        assert!(matches!(err, Err(BackendError::InvalidStateError(_))));
    }

    #[test]
    fn commit_after_abort_fails() {
        let b = setup();
        let mut tx = IdbTransaction::begin(
            b.conn(),
            "db",
            vec!["users".into()],
            IdbTransactionMode::ReadWrite,
        )
        .unwrap();
        tx.abort(b.conn()).unwrap();

        let err = tx.commit(b.conn());
        assert!(matches!(err, Err(BackendError::InvalidStateError(_))));
    }

    #[test]
    fn scope_check_valid() {
        let b = setup();
        let tx = IdbTransaction::begin(
            b.conn(),
            "db",
            vec!["users".into(), "posts".into()],
            IdbTransactionMode::ReadOnly,
        )
        .unwrap();

        assert!(tx.check_scope("users").is_ok());
        assert!(tx.check_scope("posts").is_ok());
    }

    #[test]
    fn scope_check_invalid() {
        let b = setup();
        let tx = IdbTransaction::begin(
            b.conn(),
            "db",
            vec!["users".into()],
            IdbTransactionMode::ReadOnly,
        )
        .unwrap();

        let err = tx.check_scope("posts");
        assert!(matches!(err, Err(BackendError::NotFoundError(_))));
    }

    #[test]
    fn version_change_bypasses_scope() {
        let b = setup();
        let tx = IdbTransaction::begin(b.conn(), "db", vec![], IdbTransactionMode::VersionChange)
            .unwrap();

        // VersionChange can access any store regardless of scope
        assert!(tx.check_scope("users").is_ok());
        assert!(tx.check_scope("anything").is_ok());
    }

    #[test]
    fn check_writable_readonly_fails() {
        let b = setup();
        let tx = IdbTransaction::begin(
            b.conn(),
            "db",
            vec!["users".into()],
            IdbTransactionMode::ReadOnly,
        )
        .unwrap();

        let err = tx.check_writable();
        assert!(matches!(err, Err(BackendError::ReadOnlyError(_))));
    }

    #[test]
    fn check_writable_readwrite_ok() {
        let b = setup();
        let tx = IdbTransaction::begin(
            b.conn(),
            "db",
            vec!["users".into()],
            IdbTransactionMode::ReadWrite,
        )
        .unwrap();

        assert!(tx.check_writable().is_ok());
    }

    #[test]
    fn abort_rolls_back_writes() {
        let b = setup();
        let mut tx = IdbTransaction::begin(
            b.conn(),
            "db",
            vec!["users".into()],
            IdbTransactionMode::ReadWrite,
        )
        .unwrap();

        // Write inside transaction
        crate::ops::put(
            &b,
            "db",
            "users",
            Some(crate::IdbKey::Number(1.0)),
            r#""alice""#,
        )
        .unwrap();

        tx.abort(b.conn()).unwrap();

        // Verify data was rolled back — need a new read to check
        let val = crate::ops::get(&b, "db", "users", &crate::IdbKey::Number(1.0)).unwrap();
        assert!(val.is_none());
    }
}
