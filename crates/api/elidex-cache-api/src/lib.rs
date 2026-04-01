//! Cache API storage backend for elidex (WHATWG Cache API).
//!
//! Provides storage operations for the Cache API, built on top of
//! `elidex-storage-core`'s `SqliteConnection`. Each origin gets
//! isolated cache storage via `OriginStorageManager`.
//!
//! # Architecture
//!
//! - `storage` module: `CacheStorage`-level operations (open/has/delete/keys)
//! - `store` module: Per-cache operations (put/match/delete/keys/add_all)
//! - `entry` module: `CachedEntry` type with serialization and matching

pub mod entry;
pub mod error;
pub mod storage;
pub mod store;

pub use entry::{CachedEntry, MatchOptions};
pub use error::CacheError;
