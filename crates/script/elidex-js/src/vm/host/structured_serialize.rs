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
//! ([`natives_json::stringify_to_string`](super::super::natives_json::stringify_to_string)
//! — see [`super::worker_scope::serialize_message`]) — one encoder, two thin
//! wrappers (`String` for the worker channel, `Vec<u8>` for history storage).
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
/// Interim JSON-shortcut (UTF-8 JSON). The interim is **total — it never throws**;
/// a representable value → `Some(bytes)`, anything else → `None` (no restorable
/// state; a cross-document traversal / reload restores `null`). `JSON.stringify`'s
/// error set does not match `StructuredSerializeForStorage`'s in either direction,
/// so every mismatch degrades rather than surfacing as a `pushState` abort. The
/// spec-fidelity gaps this leaves — all closed by the full structured-clone walker
/// (`#11-history-state-structured-serialize-fidelity`), at which point the noted
/// tests flip as visible landing signals — are:
///
/// - **BigInt / cyclic**: structured clone **succeeds** (both cloneable) but
///   `JSON.stringify` throws. Degrade to `None` rather than throw
///   `DataCloneError`, which would regress `pushState({v: 10n})` etc. that browsers
///   accept (CR-3).
/// - **Date**: the one cloneable kind `JSON.stringify` does *not* fail on — it
///   renders a Date through `toJSON` as an ISO String (ECMA-262 §21.4.4.37), so a
///   traversal would silently restore a **String** where structured clone restores
///   a Date. Caught **inside** the walk by
///   [`stringify_for_structured_shortcut`]
///   (before the `toJSON` hook, so an accessor-returned Date is caught too) and
///   degraded to `None` here — the same policy as BigInt / cyclic, for the same
///   CR-3 reason (a `pushState` that StructuredSerializeForStorage accepts must not start throwing).
/// - **`function` / `symbol`**: structured clone must throw `DataCloneError`, but
///   `JSON.stringify` renders them as `undefined` → `None`. The opposite-direction
///   gap (succeeds where the spec throws).
/// - **`undefined`**: a primitive that `StructuredSerializeInternal` round-trips as
///   `undefined` (§2.7.4 step 5), but JSON cannot encode → collapses to `None`
///   (restores `null`). Preserving it needs a tagged non-JSON encoding = the walker
///   slot's codec, so it is NOT special-cased here (Codex R5; avoids a bespoke
///   `undefined` sentinel over the "UTF-8 JSON" wire — One-issue-one-way).
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
