//! IndexedDB value / key / error marshalling (W3C IDB §7 ECMAScript
//! binding + §5.11 clone + §3 Exceptions).
//!
//! Marshalling only (CLAUDE.md Layering mandate): `JsValue` ↔ `IdbKey` /
//! value conversion + `BackendError` → `DOMException` mapping.  All key
//! encoding / ordering / key-path evaluation lives in the
//! `elidex-indexeddb` backend (`key.rs` / `ops.rs`).
//!
//! Note: this VM has no `Date` object, so `Date` keys (backend
//! `IdbKey::Date`) cannot be produced from JS — keys are Number / String /
//! Array (§7.4).  `Date`-key support is orthogonal, gated on a future
//! `Date` builtin.

#![cfg(feature = "engine")]

use super::super::super::value::{JsValue, NativeContext, ObjectId, ObjectKind, VmError};
use super::super::super::VmInner;

/// Maximum nested-array key depth (matches the backend `key.rs` bound).
const MAX_KEY_DEPTH: usize = 64;

/// A query argument that is either a single key or an `IDBKeyRange`
/// (W3C IDB §4.5 `get` / `getAll` / `count` / `delete` accept both).
pub(crate) enum Query {
    Key(elidex_indexeddb::IdbKey),
    Range(elidex_indexeddb::IdbKeyRange),
}

/// Map a backend [`elidex_indexeddb::BackendError`] to a `DOMException`
/// wrapper `ObjectId` with the spec-mandated `name` (W3C IDB §3).
pub(super) fn backend_error_to_dom_exception(
    vm: &mut VmInner,
    err: &elidex_indexeddb::BackendError,
) -> ObjectId {
    let name_sid = vm.strings.intern(err.dom_exception_name());
    let message = err.to_string();
    match vm.build_dom_exception(name_sid, &message) {
        JsValue::Object(id) => id,
        _ => unreachable!("build_dom_exception returned a non-object"),
    }
}

/// Build a synchronously-thrown `DOMException`-tagged [`VmError`] (e.g.
/// `DataError` for an invalid key, `TransactionInactiveError`).
pub(crate) fn dom_exc(
    ctx: &mut NativeContext<'_>,
    name: &str,
    message: impl Into<String>,
) -> VmError {
    let sid = ctx.vm.strings.intern(name);
    VmError::dom_exception(sid, message.into())
}

/// Map a [`elidex_indexeddb::BackendError`] to a synchronously-thrown
/// `DOMException` [`VmError`] — for schema operations (`createObjectStore`
/// / `deleteObjectStore`) that throw directly rather than returning a
/// request (W3C IDB §4.4 / §4.5).
pub(crate) fn backend_error_as_throw(
    ctx: &mut NativeContext<'_>,
    err: &elidex_indexeddb::BackendError,
) -> VmError {
    dom_exc(ctx, err.dom_exception_name(), err.to_string())
}

/// W3C IDB §7.4 "convert a value to a key": Number / String / Array →
/// `IdbKey`; anything else (undefined / null / boolean / NaN / object) →
/// `DataError`.
pub(crate) fn js_to_idb_key(
    ctx: &mut NativeContext<'_>,
    val: JsValue,
) -> Result<elidex_indexeddb::IdbKey, VmError> {
    js_to_idb_key_depth(ctx, val, 0)
}

fn js_to_idb_key_depth(
    ctx: &mut NativeContext<'_>,
    val: JsValue,
    depth: usize,
) -> Result<elidex_indexeddb::IdbKey, VmError> {
    if depth > MAX_KEY_DEPTH {
        return Err(dom_exc(
            ctx,
            "DataError",
            "key array nesting exceeds the maximum depth",
        ));
    }
    match val {
        JsValue::Number(n) => {
            if n.is_nan() {
                return Err(dom_exc(ctx, "DataError", "NaN is not a valid key"));
            }
            Ok(elidex_indexeddb::IdbKey::Number(n))
        }
        JsValue::String(sid) => Ok(elidex_indexeddb::IdbKey::String(ctx.get_utf8(sid))),
        JsValue::Object(id) => {
            let elements = match &ctx.get_object(id).kind {
                ObjectKind::Array { elements } => Some(elements.clone()),
                _ => None,
            };
            match elements {
                Some(elems) => {
                    let mut keys = Vec::with_capacity(elems.len());
                    for elem in elems {
                        // Holes / undefined inside a key array are invalid keys.
                        keys.push(js_to_idb_key_depth(ctx, elem, depth + 1)?);
                    }
                    Ok(elidex_indexeddb::IdbKey::Array(keys))
                }
                None => Err(dom_exc(ctx, "DataError", "value is not a valid key")),
            }
        }
        _ => Err(dom_exc(ctx, "DataError", "value is not a valid key")),
    }
}

/// W3C IDB §7.3 "convert a key to a value": `IdbKey` → `JsValue`.  `Date`
/// keys (which JS cannot currently produce) degrade to their numeric ms
/// value.
pub(crate) fn idb_key_to_js(vm: &mut VmInner, key: &elidex_indexeddb::IdbKey) -> JsValue {
    match key {
        elidex_indexeddb::IdbKey::Number(n) | elidex_indexeddb::IdbKey::Date(n) => {
            JsValue::Number(*n)
        }
        elidex_indexeddb::IdbKey::String(s) => JsValue::String(vm.strings.intern(s)),
        elidex_indexeddb::IdbKey::Array(items) => {
            let elems: Vec<JsValue> = items.iter().map(|k| idb_key_to_js(vm, k)).collect();
            JsValue::Object(vm.create_array_object(elems))
        }
    }
}

/// Resolve a query argument to a single key or an `IDBKeyRange` (§4.5).
pub(crate) fn js_to_query(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<Query, VmError> {
    if let JsValue::Object(id) = val {
        if matches!(ctx.get_object(id).kind, ObjectKind::IdbKeyRange) {
            let range = ctx
                .vm
                .idb_key_range_states
                .get(&id)
                .cloned()
                .ok_or_else(|| VmError::type_error("IDBKeyRange state missing"))?;
            return Ok(Query::Range(range));
        }
    }
    Ok(Query::Key(js_to_idb_key(ctx, val)?))
}

/// W3C IDB §5.11 "clone a value" + value-storage serialization.  v1
/// persists JSON-representable values only (the backend stores
/// `serde_json` TEXT); non-representable structured types (Function /
/// Symbol / etc.) → `DataCloneError`.  Full structured-clone fidelity is
/// deferred to `#11-idb-structured-clone-storage`.
///
/// The §5.11 transaction-inactive-during-clone guard (so a getter side
/// effect can't re-enter the transaction) is applied by the caller around
/// this call (it owns the transaction id).
pub(crate) fn value_to_json(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<String, VmError> {
    match super::super::super::natives_json::stringify_to_string(
        ctx,
        val,
        JsValue::Undefined,
        JsValue::Undefined,
    )? {
        Some(s) => Ok(s),
        // JSON.stringify → undefined (e.g. a bare function / undefined):
        // not storable.  §5.11 surfaces non-clonable input as DataCloneError.
        None => Err(dom_exc(
            ctx,
            "DataCloneError",
            "value could not be cloned for IndexedDB storage (not JSON-representable)",
        )),
    }
}

/// Deserialize a backend JSON value blob back to a `JsValue` (read path).
pub(crate) fn json_to_js(vm: &mut VmInner, json: &str) -> JsValue {
    super::super::super::natives_json::parse_json_str(vm, json).unwrap_or(JsValue::Undefined)
}
