use std::fmt;

/// Storage error kind.
#[derive(Debug)]
pub enum StorageErrorKind {
    /// Database not found or cannot be opened.
    NotFound,
    /// Constraint violation (unique, foreign key, etc.).
    Constraint,
    /// I/O error (disk full, permission denied, etc.).
    Io,
    /// Schema migration failed.
    Migration,
    /// Quota exceeded.
    QuotaExceeded,
    /// Underlying SQLite error.
    Sqlite,
    /// Other unclassified error.
    Other,
}

/// Error from storage operations.
#[derive(Debug)]
pub struct StorageError {
    pub kind: StorageErrorKind,
    pub message: String,
}

impl StorageError {
    pub fn new(kind: StorageErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StorageErrorKind::NotFound, message)
    }

    pub fn constraint(message: impl Into<String>) -> Self {
        Self::new(StorageErrorKind::Constraint, message)
    }

    pub fn quota_exceeded(message: impl Into<String>) -> Self {
        Self::new(StorageErrorKind::QuotaExceeded, message)
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StorageError({:?}): {}", self.kind, self.message)
    }
}

impl std::error::Error for StorageError {}

impl From<rusqlite::Error> for StorageError {
    fn from(e: rusqlite::Error) -> Self {
        let kind = match &e {
            rusqlite::Error::QueryReturnedNoRows => StorageErrorKind::NotFound,
            rusqlite::Error::SqliteFailure(err, _)
                if err.code == rusqlite::ffi::ErrorCode::ConstraintViolation =>
            {
                StorageErrorKind::Constraint
            }
            _ => StorageErrorKind::Sqlite,
        };
        Self::new(kind, e.to_string())
    }
}
