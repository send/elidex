//! `FileReader` interface (File API §6) — async byte reader for `Blob`
//! / `File` instances.
//!
//! ```text
//! FileReader instance (ObjectKind::FileReader, payload-free)
//!   → FileReader.prototype  (this module)
//!     → EventTarget.prototype  (vm/host/event_target.rs)
//! ```
//!
//! ## Scope
//!
//! - `readAsText(blob, encoding?)` / `readAsArrayBuffer(blob)` /
//!   `readAsDataURL(blob)` / `readAsBinaryString(blob)` — set state to
//!   LOADING, fire `loadstart` synchronously, then enqueue a
//!   [`super::pending_tasks::PendingTask::FileRead`] task whose drain
//!   performs the actual read + fires terminal events.
//! - `abort()` — cancels an in-flight read by incrementing `abort_seq`
//!   in the side-data; the drained task compares its snapshot to the
//!   current value and silent-discards on mismatch.
//! - `readyState` / `result` / `error` IDL readonly attrs.
//! - Event handler attributes: `onloadstart` / `onprogress` / `onload`
//!   / `onloadend` / `onerror` / `onabort` (per spec §6.2 IDL).
//!
//! ## Defer
//!
//! Phase 0b ships scaffolding only — `register_file_reader_global` is
//! a stub.  The actual ctor + readAs* + abort wiring + event dispatch
//! lives in Phase 4 within `#11-file-api`.

#![cfg(feature = "engine")]
// Phase 0b scaffolding — see [`super::file`] for rationale.  Removed
// when Phase 4 wires real ctor + readAs* + abort + event dispatch.
#![allow(dead_code)]

use super::super::value::ObjectId;

/// `FileReader.readyState` enum per FileAPI §6 (3 values).
///
/// Transitions: EMPTY (init) → LOADING (during a `readAs*()`
/// in-flight) → DONE (terminal — success / error / abort).  A new
/// `readAs*()` from DONE state resets to LOADING (NOT EMPTY).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum ReadyState {
    Empty = 0,
    Loading = 1,
    Done = 2,
}

/// `FileReader.result` typed payload per FileAPI §6.2.  Always `None`
/// while state != DONE; populated by the read task drain on success.
///
/// `ArrayBuffer` carries the `ObjectId` of a freshly allocated
/// ArrayBuffer wrapper (GC trace fan-out marks it).  `DataUrl` /
/// `Text` / `BinaryString` carry owned `String`s (no rooting needed —
/// String is heap-owned Rust data, not a JsValue).
#[derive(Clone, Debug, Default)]
pub(crate) enum ReaderResult {
    #[default]
    None,
    Text(String),
    ArrayBuffer(ObjectId),
    DataUrl(String),
    BinaryString(String),
}

/// Read operation kind carried by
/// [`super::pending_tasks::PendingTask::FileRead`] so the dispatch
/// step knows which decode path to take.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReadKind {
    Text,
    ArrayBuffer,
    DataUrl,
    BinaryString,
}

/// Per-`FileReader` out-of-band state, keyed in
/// [`super::super::VmInner::file_reader_data`] by the instance's
/// `ObjectId`.
///
/// Default: state=Empty, no result, no error, no target, abort_seq=0.
///
/// `abort_seq` is incremented on every `abort()` AND on every new
/// `readAs*()` call.  Enqueued `PendingTask::FileRead` carries a
/// snapshot of `abort_seq` at enqueue time; the drain compares and
/// silent-discards if mismatched.  u32 wrap-around requires ~4 billion
/// abort/read cycles per single FileReader instance — practically
/// impossible (typical JS code triggers a handful per page lifetime).
#[derive(Debug)]
pub(crate) struct FileReaderSideData {
    pub(crate) state: ReadyState,
    pub(crate) result: ReaderResult,
    /// `DOMException` wrapper `ObjectId` (e.g. NotReadableError,
    /// AbortError) populated on error / abort outcomes.  `None`
    /// while state != DONE OR successful.  GC traced.
    pub(crate) error: Option<ObjectId>,
    /// `ObjectId` of the Blob / File being read (the argument passed
    /// to the active `readAs*()` call).  Kept here for the drain to
    /// re-resolve `blob_data` at task-fire time.  `None` while EMPTY.
    /// GC traced.
    pub(crate) target_blob: Option<ObjectId>,
    /// Monotonic counter — increments on `abort()` AND on each
    /// `readAs*()` call.  Drain snapshots vs current to detect
    /// staleness (abort happened OR a new read superseded).
    pub(crate) abort_seq: u32,
}

impl Default for FileReaderSideData {
    fn default() -> Self {
        Self {
            state: ReadyState::Empty,
            result: ReaderResult::None,
            error: None,
            target_blob: None,
            abort_seq: 0,
        }
    }
}

impl crate::vm::VmInner {
    /// Allocate `FileReader.prototype` (chains to
    /// `EventTarget.prototype`), install accessor / method suite, and
    /// expose `FileReader` constructor on `globals` along with EMPTY
    /// / LOADING / DONE constants on both ctor and prototype per
    /// FileAPI §6.  Phase 0b stub — Phase 4 wires the real ctor +
    /// readAs* + abort + event dispatch.
    #[allow(clippy::unused_self)] // Phase 0b stub — Phase 4 reads self.
    pub(in crate::vm) fn register_file_reader_global(&mut self) {
        // TODO(#11-file-api Phase 4): allocate proto chained on
        // event_target_prototype, install readAs* + abort + on*
        // handler attributes + readyState / result / error accessors,
        // install EMPTY / LOADING / DONE constants on both ctor and
        // prototype.
    }
}
