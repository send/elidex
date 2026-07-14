//! `StructuredSerializeForStorage` / `StructuredDeserialize` **byte** seam for
//! the classic `history.state` (WHATWG HTML §7.2.5 "shared history push/replace
//! state steps" step 3 → §2.7.5 `StructuredSerializeForStorage`, and the
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

use std::collections::HashSet;

use super::super::natives_json::{parse_json_str, stringify_to_string};
use super::super::value::{JsValue, NativeContext, ObjectId, ObjectKind, PropertyValue};
use super::super::VmInner;

/// Whether `value`'s object graph contains a `Date`.
///
/// Every *other* way the JSON shortcut deviates from structured clone is **loud**:
/// a `BigInt`, a cycle, or the depth cap makes `stringify_to_string` throw, so it
/// already degrades to `None` here / maps to `DataCloneError` in
/// [`super::worker_scope::serialize_message`]. A `Date` is the one cloneable kind
/// it mis-encodes **silently** — `Date.prototype.toJSON` (ECMA-262 §21.4.4.37)
/// renders it as an ISO String, so the peer's `JSON.parse` /
/// [`structured_deserialize`] yields a String where structured clone yields a
/// Date. Detect it up front so each wrapper applies its own failure policy instead
/// of shipping the silent type change — the same stance IndexedDB takes via its
/// own `reject_non_json_storable` walker.
///
/// Walks what `JSON.stringify` walks: array elements plus the **enumerable own
/// data** properties of every non-callable object (`SerializeJSONObject` runs over
/// `EnumerableOwnProperties`, so a Date parked on an `Error` / `RegExp` is reached
/// too; a callable is omitted from JSON entirely). Accessors are skipped —
/// invoking a getter here would be an observable side effect, and
/// `reject_non_json_storable` skips them for the same reason — so a getter
/// *returning* a Date still slips through, a gap the walker slots below close.
///
/// Removed wholesale by the full structured-clone walker
/// (`#11-worker-structured-serialize` /
/// `#11-history-state-structured-serialize-fidelity`), which encodes a `Date`
/// faithfully instead of refusing it.
pub(in crate::vm) fn contains_date(vm: &VmInner, value: JsValue) -> bool {
    let mut seen = HashSet::new();
    contains_date_inner(vm, value, &mut seen)
}

fn contains_date_inner(vm: &VmInner, value: JsValue, seen: &mut HashSet<ObjectId>) -> bool {
    let JsValue::Object(id) = value else {
        return false;
    };
    if !seen.insert(id) {
        return false;
    }
    let obj = vm.get_object(id);
    match &obj.kind {
        // `toJSON` fires before any property walk, so a Date never survives as one.
        ObjectKind::Date(_) => return true,
        // `JSON.stringify` omits a callable value entirely.
        ObjectKind::Function(_)
        | ObjectKind::NativeFunction(_)
        | ObjectKind::BoundFunction { .. } => return false,
        _ => {}
    }
    let mut children: Vec<JsValue> = match &obj.kind {
        ObjectKind::Array { elements } => elements.clone(),
        _ => Vec::new(),
    };
    children.extend(
        obj.storage
            .iter_properties(&vm.shapes)
            .filter_map(|(_, val, attrs)| {
                if !attrs.enumerable || attrs.is_accessor {
                    return None;
                }
                match val {
                    PropertyValue::Data(v) => Some(*v),
                    PropertyValue::Accessor { .. } => None,
                }
            }),
    );
    children
        .into_iter()
        .any(|child| contains_date_inner(vm, child, seen))
}

/// `StructuredSerializeForStorage(value)` → **optional** storage bytes (WHATWG HTML
/// §7.2.5 step 3 / §2.7.5, `forStorage = true`).
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
///   a Date. Caught up front by [`contains_date`] and degraded to `None` — the same
///   policy as BigInt / cyclic, and for the same CR-3 reason (a `pushState` that
///   browsers accept must not start throwing).
/// - **`function` / `symbol`**: structured clone must throw `DataCloneError`, but
///   `JSON.stringify` renders them as `undefined` → `None`. The opposite-direction
///   gap (succeeds where the spec throws).
/// - **`undefined`**: a primitive that `StructuredSerializeInternal` round-trips as
///   `undefined` (§2.7.3 step 4), but JSON cannot encode → collapses to `None`
///   (restores `null`). Preserving it needs a tagged non-JSON encoding = the walker
///   slot's codec, so it is NOT special-cased here (Codex R5; avoids a bespoke
///   `undefined` sentinel over the "UTF-8 JSON" wire — One-issue-one-way).
/// - **A throwing `toJSON`**: `StructuredSerializeInternal` serializes ordinary
///   objects via enumerable-property `? Get` (§2.7.3 step 26.4, the Object branch
///   entered at step 24) and **never invokes** JSON's `toJSON` hook, so a throwing
///   `toJSON` does NOT abort real serialization. The JSON shortcut *does* call it
///   and throws — a JSON-only exception that must degrade, not propagate and lose
///   the history entry (Codex R5).
/// - **A throwing property getter**: structured clone WOULD propagate it (via
///   `? Get`, §2.7.3 step 26.4.1). The interim could tell it apart from the
///   `toJSON` throw above only by a `toJSON`-skip that partially reimplements
///   structured-clone semantics on the shared JSON encoder (which also backs
///   `Worker.postMessage`) — a half-structured-clone mode the walker slot owns
///   wholesale — so it degrades here too rather than growing that seam (deferred by
///   design, not by impossibility; a gap the walker restores).
pub(in crate::vm) fn structured_serialize_for_storage(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
) -> Option<Vec<u8>> {
    // A Date would be encoded as an ISO String rather than failing (see
    // [`contains_date`]), so a traversal would restore a String where structured
    // clone restores a Date. Degrade instead of shipping that silent type change.
    if contains_date(ctx.vm, value) {
        return None;
    }
    match stringify_to_string(ctx, value, JsValue::Undefined, JsValue::Undefined) {
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
    /// 8.4). `None` (a plain load, or the boa engine) → `null`.
    pub(crate) fn seed_history_state(&mut self, serialized_state: Option<Vec<u8>>) {
        self.navigation.current_state = match serialized_state {
            Some(bytes) => structured_deserialize(self, &bytes),
            None => JsValue::Null,
        };
    }
}
