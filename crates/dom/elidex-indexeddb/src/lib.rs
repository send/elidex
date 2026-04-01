//! `IndexedDB` storage backend for elidex.
//!
//! Implements the W3C `IndexedDB` API 3.0 data model: keys, key ranges,
//! object stores, indexes, cursors, and transactions — backed by `SQLite`
//! via `elidex-storage-core`.
//!
//! Connection lifecycle is managed by `OriginStorageManager` from
//! `elidex-storage-core`. `IdbBackend` receives a `SqliteConnection`
//! and applies IDB-specific schema.

mod backend;
pub mod cursor;
pub mod database;
pub mod index;
mod key;
mod key_range;
pub mod ops;
mod transaction;
pub(crate) mod util;

pub use backend::{BackendError, IdbBackend};
pub use database::{IdbDatabaseHandle, IdbOpenResult};
pub use index::IndexMeta;
pub use key::IdbKey;
pub use key_range::IdbKeyRange;
pub use ops::DeleteTarget;
pub use transaction::{IdbTransaction, IdbTransactionMode, TransactionState};
