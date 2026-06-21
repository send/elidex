// SQLite limit parameters use usize→i64 casts safe for practical values.
#![allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]

pub mod backend;
pub mod browser_db;
pub mod error;
pub mod migration;
pub mod origin_manager;
pub mod quota;
pub mod sqlite;
pub mod util;
/// Synchronous Web Storage backend — gated behind `feature = "web-storage"` so
/// the `elidex-app` profile drops it from the binary (A2 absence guarantee).
#[cfg(feature = "web-storage")]
pub mod web_storage;

pub use backend::{
    CustomOp, Migration, OpenOptions, StorageBackend, StorageConnection, StorageOp, StorageResult,
};
pub use browser_db::BrowserDb;
pub use error::{StorageError, StorageErrorKind};
pub use origin_manager::{OriginKey, OriginStorageManager, StorageType};
pub use quota::{QuotaEstimate, QuotaManager};
pub use sqlite::{SqliteBackend, SqliteConnection};
pub use util::sanitize_sql_name;
#[cfg(feature = "web-storage")]
pub use web_storage::{SessionStorageState, StorageArea, WebStorageManager, STORAGE_QUOTA_BYTES};
