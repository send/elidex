//! `StructuredSerializeForStorage` / `StructuredDeserialize` **byte** seam for
//! the classic `history.state` (WHATWG HTML §7.2.5 "shared history push/replace
//! state steps" step 3 → §2.7.6 `StructuredSerializeForStorage`, and the
//! *restore the history object state* step 2 `StructuredDeserialize`).
//!
//! Unlike [`super::structured_clone`] (a *fused* in-memory
//! StructuredSerialize+Deserialize producing a `JsValue`), `history.state` must
//! survive a **cross-document traversal** — a pipeline rebuild = a fresh VM — so
//! it is serialized to **storage bytes** on the engine-independent
//! `HistoryEntry` and reconstructed in the rebuilt VM.
//!
//! **Interim (JSON-shortcut)**: the bytes are UTF-8 JSON, produced by the SAME
//! core the worker `postMessage` path uses
//! ([`stringify_for_structured_shortcut`]; see [`super::worker_scope::serialize_message`]).
//! Workers send the JSON string; history stores its UTF-8 bytes.
//! Full `StructuredSerializeForStorage` (Blob / File / Map / Date / cyclic
//! graphs to storage bytes) is deferred to
//! `#11-history-state-structured-serialize-fidelity`, which folds with the
//! worker `#11-worker-structured-serialize` slot (both swap this shared core
//! body; the `Vec<u8>` field/wire is unchanged).

#![cfg(feature = "engine")]

use super::super::natives_json::{parse_json_str, stringify_for_structured_shortcut};
use super::super::value::{JsValue, NativeContext};
use super::super::VmInner;

/// `StructuredSerializeForStorage(value)` → **optional** storage bytes (WHATWG HTML
/// §7.2.5 step 3 / §2.7.6, `forStorage = true`).
///
/// The interim UTF-8 JSON encoder returns bytes on success and `None` on
/// failure or absent output. Successful JSON encoding can still lose ordinary
/// properties. Full storage serialization is tracked in
/// `#11-history-state-structured-serialize-fidelity`.
///
/// - **BigInt / cyclic**: structured clone **succeeds** (both cloneable) but
///   `JSON.stringify` throws. Degrade to `None` rather than throw
///   `DataCloneError`, which would regress `pushState({v: 10n})` etc. that browsers
///   accept (CR-3).
/// - The encoder rejects a Date encountered during the walk, before its
///   `toJSON` hook could replace it with a string or null.
/// - A symbol or a value that remains callable after the `toJSON` hook yields
///   no JSON text at the top level. An ordinary object property with such a
///   value is omitted, so history can still store bytes for the enclosing object.
/// - Top-level `undefined` produces no bytes here. As an ordinary property
///   value it can disappear from JSON; HTML §2.7.4 step 5 preserves it.
/// - On an ordinary object, HTML §2.7.4 step 28.4 reads each still-own
///   enumerable string key. A throwing getter propagates; a callable value
///   reaches step 22 and throws DataCloneError. JSON can invoke a `toJSON`
///   method instead. History discards the bytes when the encoder throws.
pub(in crate::vm) fn structured_serialize_for_storage(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
) -> Option<Vec<u8>> {
    match stringify_for_structured_shortcut(ctx, value) {
        Ok(Some(json)) => Some(json.into_bytes()),
        // Everything else degrades to no restorable state: a top-level value JSON
        // renders as `undefined` (`function` / `symbol` / `undefined`), a
        // representability failure (cyclic / `BigInt` / depth cap), OR a user
        // exception thrown during serialization (throwing `toJSON` / getter). See
        // the per-case rationale above — all restored to fidelity by the walker slot.
        _ => None,
    }
}

/// `StructuredDeserialize(bytes)` → `JsValue` (the *restore the history object
/// state* step 2). Per that step, "If this throws an exception, catch it and let
/// state be null" — so **any** decode failure (non-UTF-8 bytes or a JSON parse
/// error) yields [`JsValue::Null`], never an error.
pub(in crate::vm) fn structured_deserialize(vm: &mut VmInner, bytes: &[u8]) -> JsValue {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return JsValue::Null;
    };
    parse_json_str(vm, text).unwrap_or(JsValue::Null)
}

impl VmInner {
    /// Seed `history.state` from a session-history entry's serialized state
    /// (WHATWG HTML §7.4.6.2 step 6.3 "restore the history object state",
    /// **without** firing popstate). Used by `HostDriver::set_history_state` at
    /// document construction so a **cross-document traversal**'s rebuilt document
    /// reads the restored `history.state` before its initial scripts run (step
    /// 8.4). Absent bytes, including after a VM encoding failure, restore `null`.
    pub(crate) fn seed_history_state(&mut self, serialized_state: Option<Vec<u8>>) {
        self.navigation.current_state = match serialized_state {
            Some(bytes) => structured_deserialize(self, &bytes),
            None => JsValue::Null,
        };
    }
}
