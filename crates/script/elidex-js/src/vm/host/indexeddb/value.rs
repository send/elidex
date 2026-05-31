//! IndexedDB value / key / error marshalling (W3C IDB §7 ECMAScript
//! binding + §3 Exceptions).
//!
//! Marshalling only (CLAUDE.md Layering mandate): `JsValue` ↔ `IdbKey` /
//! value conversion + `BackendError` → `DOMException` mapping.  All key
//! encoding / ordering / key-path evaluation lives in the
//! `elidex-indexeddb` backend (`key.rs` / `ops.rs`).
//!
//! Key / value conversion (`js_to_idb_key` / `idb_key_to_js` /
//! `clone_for_store`, §7.1–§7.4 / §5.11) lands in Stage 4 alongside the
//! CRUD surface that consumes it.  Stage 2 ships the error mapper that the
//! request lifecycle needs.

#![cfg(feature = "engine")]

use super::super::super::value::{JsValue, ObjectId};
use super::super::super::VmInner;

/// Map a backend [`elidex_indexeddb::BackendError`] to a `DOMException`
/// wrapper `ObjectId` with the spec-mandated `name` (W3C IDB §3 maps each
/// failure to a named `DOMException`; the backend's `dom_exception_name`
/// is the single source of truth for that mapping).
pub(super) fn backend_error_to_dom_exception(
    vm: &mut VmInner,
    err: &elidex_indexeddb::BackendError,
) -> ObjectId {
    let name_sid = vm.strings.intern(err.dom_exception_name());
    let message = err.to_string();
    match vm.build_dom_exception(name_sid, &message) {
        JsValue::Object(id) => id,
        // `build_dom_exception` always allocates a DOMException object.
        _ => unreachable!("build_dom_exception returned a non-object"),
    }
}
