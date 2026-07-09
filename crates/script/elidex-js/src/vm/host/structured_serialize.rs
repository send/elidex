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

use super::super::natives_json::{parse_json_str, stringify_to_string};
use super::super::value::{JsValue, NativeContext, VmError, VmErrorKind};
use super::super::VmInner;

/// `StructuredSerializeForStorage(value)` → **optional** storage bytes (WHATWG HTML
/// §7.2.5 step 3 / §2.7.5, `forStorage = true`).
///
/// Interim JSON-shortcut (UTF-8 JSON). Because `JSON.stringify`'s error set does
/// NOT match `StructuredSerializeForStorage`'s in either direction (structured
/// clone **succeeds** for BigInt / cyclic / Map / Date, which JSON throws on; and
/// JSON silently drops functions / symbols, which structured clone must throw
/// `DataCloneError` on), the interim **never throws for a representability
/// failure** — it **degrades to `Ok(None)`** (no restorable state; a cross-document
/// traversal restores `null`). Throwing `DataCloneError` for a JSON-unrepresentable
/// value would regress `pushState({v: 10n})` etc., which browsers accept (CR-3).
/// The opposite deviation (a `function` / `symbol` succeeding with null state where
/// the spec mandates `DataCloneError`) is a distinct interim gap, both fixed only
/// by the full structured-clone walker (`#11-history-state-structured-serialize-fidelity`).
///
/// Only a **user exception thrown *during* serialization** — a throwing `toJSON` /
/// property getter (a [`VmErrorKind::ThrowValue`], matching the shared push/replace
/// steps step 3 "Rethrow any exceptions") — propagates as `Err`. A representable
/// value → `Ok(Some(bytes))`; anything JSON cannot represent → `Ok(None)`.
pub(in crate::vm) fn structured_serialize_for_storage(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
) -> Result<Option<Vec<u8>>, VmError> {
    // The two `Ok(None)` outcomes are semantically distinct (an `undefined`-ish
    // top-level value vs a representability failure) but share the same degrade
    // result; kept as separate documented arms.
    #[allow(clippy::match_same_arms)]
    match stringify_to_string(ctx, value, JsValue::Undefined, JsValue::Undefined) {
        Ok(Some(json)) => Ok(Some(json.into_bytes())),
        // A top-level value `JSON.stringify` renders as `undefined` (a `function` /
        // `symbol` / `undefined`) → no restorable state (restores `null`).
        Ok(None) => Ok(None),
        // A user exception thrown during serialization propagates unchanged.
        Err(e) if matches!(e.kind, VmErrorKind::ThrowValue(_)) => Err(e),
        // A representability failure (circular / `BigInt` / depth cap) — all
        // structured-cloneable, so degrade rather than throw (CR-3).
        Err(_) => Ok(None),
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
