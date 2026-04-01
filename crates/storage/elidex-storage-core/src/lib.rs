// SQLite limit parameters use usize→i64 casts safe for practical values.
#![allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]

pub mod backend;
pub mod error;
pub mod migration;
pub mod origin_manager;
pub mod quota;
pub mod sqlite;
pub mod util;

pub use backend::{
    CustomOp, Migration, OpenOptions, StorageBackend, StorageConnection, StorageOp, StorageResult,
};
pub use error::{StorageError, StorageErrorKind};
pub use origin_manager::{OriginKey, OriginStorageManager, StorageType};
pub use quota::{QuotaEstimate, QuotaManager};
pub use sqlite::{SqliteBackend, SqliteConnection};
pub use util::sanitize_sql_name;
