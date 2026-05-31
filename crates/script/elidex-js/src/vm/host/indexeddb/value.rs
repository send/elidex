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

/// Whether a WTF-16 code-unit sequence contains an unpaired (lone) surrogate.
/// Such strings have no UTF-8 representation, so they cannot be a backend
/// string key without aliasing (see [`js_to_idb_key`]'s String arm).
fn has_unpaired_surrogate(units: &[u16]) -> bool {
    let mut i = 0;
    while i < units.len() {
        let u = units[i];
        if (0xD800..=0xDBFF).contains(&u) {
            // High surrogate: must be immediately followed by a low surrogate.
            if i + 1 >= units.len() || !(0xDC00..=0xDFFF).contains(&units[i + 1]) {
                return true;
            }
            i += 2;
        } else if (0xDC00..=0xDFFF).contains(&u) {
            // Low surrogate with no preceding high surrogate.
            return true;
        } else {
            i += 1;
        }
    }
    false
}

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
    match val {
        JsValue::Number(n) => {
            if n.is_nan() {
                return Err(dom_exc(ctx, "DataError", "NaN is not a valid key"));
            }
            Ok(elidex_indexeddb::IdbKey::Number(n))
        }
        JsValue::String(sid) => {
            // The backend stores string keys as UTF-8, so a lossy WTF-16→UTF-8
            // conversion of an unpaired surrogate (→ U+FFFD) would alias
            // distinct keys.  Reject such keys with `DataError` rather than
            // silently aliasing — same defensive stance as the `MAX_KEY_DEPTH`
            // array-depth rejection below (a faithful WTF-16 key store is
            // deferred to `#11-idb-binary-key`).
            if has_unpaired_surrogate(ctx.get_u16(sid)) {
                return Err(dom_exc(
                    ctx,
                    "DataError",
                    "string key contains an unpaired surrogate (not representable)",
                ));
            }
            Ok(elidex_indexeddb::IdbKey::String(ctx.get_utf8(sid)))
        }
        JsValue::Object(id) => {
            let elements = match &ctx.get_object(id).kind {
                ObjectKind::Array { elements } => Some(elements.clone()),
                _ => None,
            };
            match elements {
                Some(elems) => {
                    // The backend serializes a nested array key only while its
                    // depth is `< MAX_KEY_DEPTH`; a deeper array is silently
                    // serialized as empty (truncated), which would alias
                    // distinct keys.  Reject here so the backend never
                    // truncates (matches `elidex_indexeddb` `key.rs`
                    // `serialize_into`'s `depth < MAX_KEY_DEPTH` guard).
                    if depth >= MAX_KEY_DEPTH {
                        return Err(dom_exc(
                            ctx,
                            "DataError",
                            "key array nesting exceeds the maximum depth",
                        ));
                    }
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
        // §7.4 also admits Date and buffer-source (ArrayBuffer / typed-array /
        // DataView) keys.  Date keys need a VM `Date` builtin (see the module
        // note); binary keys need a backend `IdbKey::Binary` variant
        // (`elidex-indexeddb` `key.rs` reserves `TAG_BINARY` but rejects it) —
        // deferred to `#11-idb-binary-key`.  Until then both → DataError.
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
    // §5.11 clone: ANY failure to serialize the value for storage surfaces as
    // `DataCloneError`, never the raw `JSON.stringify` exception — a cyclic
    // structure throws `TypeError`, but for IDB the value simply cannot be
    // stored by this v1 JSON clone path (full structured-clone fidelity →
    // `#11-idb-structured-clone-storage`), which is a clone failure.
    let Ok(serialized) = super::super::super::natives_json::stringify_to_string(
        ctx,
        val,
        JsValue::Undefined,
        JsValue::Undefined,
    ) else {
        return Err(dom_exc(
            ctx,
            "DataCloneError",
            "value could not be cloned for IndexedDB storage",
        ));
    };
    match serialized {
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

/// WebIDL required-argument arity check — overload resolution rejects a call
/// with too few arguments with a `TypeError` *before* any coercion runs
/// (`open()`, `store.add()`, `IDBKeyRange.only()`, … all have a required
/// first argument).  Returns the argument at `index`; an explicit `undefined`
/// at that position IS a supplied argument and is returned for normal
/// coercion.  `required` is the operation's total required-argument count
/// (for the diagnostic message).
pub(crate) fn require_arg(
    args: &[JsValue],
    index: usize,
    interface: &str,
    method: &str,
    required: usize,
) -> Result<JsValue, VmError> {
    args.get(index).copied().ok_or_else(|| {
        VmError::type_error(format!(
            "Failed to execute '{method}' on '{interface}': \
             {required} argument{} required, but only {} present.",
            if required == 1 { "" } else { "s" },
            args.len()
        ))
    })
}

/// Deserialize a backend JSON value blob back to a `JsValue` (read path).
pub(crate) fn json_to_js(vm: &mut VmInner, json: &str) -> JsValue {
    super::super::super::natives_json::parse_json_str(vm, json).unwrap_or(JsValue::Undefined)
}
