use std::fmt;

/// Cache API error types.
#[derive(Debug)]
pub enum CacheError {
    /// Cache with the given name not found.
    NotFound(String),
    /// Storage backend error.
    Storage(elidex_storage_core::StorageError),
    /// Invalid request/response data.
    Invalid(String),
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "CacheError(NotFound): {msg}"),
            Self::Storage(e) => write!(f, "CacheError(Storage): {e}"),
            Self::Invalid(msg) => write!(f, "CacheError(Invalid): {msg}"),
        }
    }
}

impl std::error::Error for CacheError {}

impl From<elidex_storage_core::StorageError> for CacheError {
    fn from(e: elidex_storage_core::StorageError) -> Self {
        Self::Storage(e)
    }
}

impl From<rusqlite::Error> for CacheError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Storage(elidex_storage_core::StorageError::from(e))
    }
}
