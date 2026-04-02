// Script hash and persistence use u64↔i64 casts for SQLite compatibility.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

//! Service Worker lifecycle, Cache API integration, and PWA support for elidex.
//!
//! Implements the WHATWG Service Worker specification:
//! - SW lifecycle state machine (parsed → installing → installed → activating → activated)
//! - Fetch event interception (FetchEvent, respondWith, waitUntil)
//! - SW registration persistence (WHATWG SW §3.1 "MUST persistently store")
//! - Scope matching (longest prefix match, WHATWG SW §8.1)
//! - Update check (byte-for-byte, 24h soft update, WHATWG SW §4.4.4)
//! - Security validation (HTTPS-only, same-origin, MIME type, Service-Worker-Allowed)
//! - Background Sync (WICG: one-shot + periodic)
//! - Web App Manifest (W3C)
//!
//! # Architecture
//!
//! The SW thread is managed by `SwHandle` (parent-side IPC channel + JoinHandle).
//! `SwRegistrationStore` tracks registrations in memory.
//! `SwPersistence` persists registrations to SQLite via `StorageBackend`.
//! All IPC uses `ContentToSw` / `SwToContent` message enums.

pub mod handle;
pub mod manifest;
pub mod persistence;
pub mod registration;
pub mod router;
pub mod scope;
pub mod security;
pub mod sync;
pub mod types;
pub mod update;

pub use handle::SwHandle;
pub use manifest::{parse_manifest, DisplayMode, ManifestIcon, ManifestShortcut, WebAppManifest};
pub use persistence::SwPersistence;
pub use registration::{SwRegistration, SwRegistrationStore, SwState, UpdateViaCache};
pub use router::{evaluate_routes, RouterCondition, RouterRule, RouterSource, UrlPattern};
pub use scope::{default_scope, find_registration, matches_scope};
pub use security::{is_secure_context, validate_mime_type, validate_registration};
pub use sync::{SyncManager, SyncRegistration, SyncState};
pub use types::{ContentToSw, LifecycleEvent, SwRequest, SwResponse, SwToContent};
pub use update::{hash_script, scripts_differ, UpdateChecker, UpdateResult};
