//! IndexedDB value / key / error marshalling (W3C IDB §7 ECMAScript
//! binding + §5.11 clone + §3 Exceptions).
//!
//! Marshalling only (CLAUDE.md Layering mandate): `JsValue` ↔ `IdbKey` /
//! value conversion + `BackendError` → `DOMException` mapping.  All key
//! encoding / ordering / key-path evaluation lives in the
//! `elidex-indexeddb` backend (`key.rs` / `ops.rs`).
//!
//! Note: keys are Number / String / Array / Date (§7.4).  A `Date` with a
//! finite `[[DateValue]]` converts to the backend `IdbKey::Date`; buffer-source
//! keys (backend `IdbKey::Binary`) are deferred to `#11-idb-binary-key`.

#![cfg(feature = "engine")]

use super::super::super::value::{
    JsValue, NativeContext, ObjectId, ObjectKind, PropertyValue, VmError,
};
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

/// Classify an existence-probe error from a store / index liveness guard: a
/// `NotFoundError` means the store / index was deleted (the §4.5 / §4.6
/// "has been deleted" → `InvalidStateError` case, with `deleted_msg`); any
/// OTHER backend error is a genuine failure (`UnknownError`, …) and must
/// surface as itself, never be masked as a deletion.
pub(super) fn deleted_or_throw(
    ctx: &mut NativeContext<'_>,
    err: &elidex_indexeddb::BackendError,
    deleted_msg: &str,
) -> VmError {
    match err {
        elidex_indexeddb::BackendError::NotFoundError(_) => {
            dom_exc(ctx, "InvalidStateError", deleted_msg.to_string())
        }
        other => backend_error_as_throw(ctx, other),
    }
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
            // §7.4: a Date with a finite [[DateValue]] converts to an
            // `IdbKey::Date`; an Invalid Date (NaN) is not a valid key.
            if let ObjectKind::Date(t) = ctx.get_object(id).kind {
                if t.is_nan() {
                    return Err(dom_exc(ctx, "DataError", "Invalid Date is not a valid key"));
                }
                return Ok(elidex_indexeddb::IdbKey::Date(t));
            }
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
        // §7.4 also admits buffer-source (ArrayBuffer / typed-array / DataView)
        // keys, which need a backend `IdbKey::Binary` variant (`elidex-indexeddb`
        // `key.rs` reserves `TAG_BINARY` but rejects it) — deferred to
        // `#11-idb-binary-key`.  (Date keys are handled in the Object arm above.)
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
/// Walk the structured-cloned graph and reject any object kind that is
/// cloneable but NOT faithfully JSON-representable (the v1 IDB storage format
/// is `serde_json` TEXT).  Such kinds would be silently corrupted by
/// `JSON.stringify` (→ `{}` / index-keyed object), so they must throw
/// `DataCloneError` until `#11-idb-structured-clone-storage` lands.  Operates
/// on the CLONE (plain data — no accessors), so traversal runs no user hooks.
/// `seen` breaks reference cycles: a back-edge is left for `JSON.stringify` to
/// reject as a circular structure (also surfaced as `DataCloneError`).
fn reject_non_json_storable(
    ctx: &mut NativeContext<'_>,
    value: JsValue,
    seen: &mut std::collections::HashSet<ObjectId>,
) -> Result<(), VmError> {
    // Classify into an owned result first so the `&Object` borrow is released
    // before `dom_exc` / the recursion re-borrow `ctx`.
    enum Kind {
        Container(Vec<JsValue>),
        Ordinary,
        Reject(&'static str),
        Leaf,
    }
    let id = match value {
        JsValue::Object(id) => id,
        // §5.11 v1 JSON storage: these structured-cloneable PRIMITIVES are not
        // faithfully JSON-representable, so `JSON.stringify` would silently
        // corrupt them — a nested `undefined` property is dropped, `NaN` /
        // `±Infinity` become `null`, and a `BigInt` throws.  Reject to match
        // both the no-silent-corruption stance and the existing top-level
        // `undefined` rejection (the `None` arm of `value_to_json`).  `-0` is
        // intentionally NOT rejected: JSON stores it as `0` and `-0 === 0`, a
        // negligible loss (faithful preservation is part of
        // `#11-idb-structured-clone-storage`).
        JsValue::Undefined => return Err(reject_unstorable(ctx, "undefined")),
        JsValue::Number(n) if !n.is_finite() => {
            let label = if n.is_nan() {
                "NaN"
            } else if n > 0.0 {
                "Infinity"
            } else {
                "-Infinity"
            };
            return Err(reject_unstorable(ctx, label));
        }
        JsValue::BigInt(_) => return Err(reject_unstorable(ctx, "a BigInt")),
        // Faithfully JSON-representable primitives (null / boolean / string /
        // finite number, incl. `-0`) are leaves.
        _ => return Ok(()),
    };
    if !seen.insert(id) {
        return Ok(());
    }
    let obj = ctx.vm.get_object(id);
    let proto = obj.prototype;
    let kind = match &obj.kind {
        // JSON-representable containers: recurse their stored data values.
        // (An Array serializes only its indexed elements, matching
        // `JSON.stringify`; extra string-keyed props on an array are ignored.)
        ObjectKind::Array { elements } => Kind::Container(elements.clone()),
        // An ordinary object is JSON-representable UNLESS it is error-like (its
        // meaningful state — `message` / `name` — lives in non-enumerable
        // properties that `JSON.stringify` drops); decided below via `proto`.
        ObjectKind::Ordinary => Kind::Ordinary,
        // Cloneable but not JSON-representable → silent corruption under
        // `JSON.stringify`.  Reject upfront with the spec-correct type name.
        ObjectKind::Error { .. } => Kind::Reject("an Error"),
        // A Date is [Serializable] but not JSON-representable: `JSON.stringify`
        // maps it through `toJSON` to an ISO string, so a stored-then-read value
        // would come back a string, not a Date.  Reject until binary
        // structured-clone storage lands (`#11-idb-binary-key` cohort).
        ObjectKind::Date(_) => Kind::Reject("a Date"),
        ObjectKind::RegExp { .. } => Kind::Reject("a RegExp"),
        ObjectKind::ArrayBuffer => Kind::Reject("an ArrayBuffer"),
        ObjectKind::Blob => Kind::Reject("a Blob"),
        ObjectKind::TypedArray { .. } => Kind::Reject("a typed array"),
        ObjectKind::DataView { .. } => Kind::Reject("a DataView"),
        // Primitive wrappers (Number/String/Boolean) JSON-serialize as their
        // primitive; everything else either can't reach here (unclonable, so
        // `clone_value` already threw) or is a leaf.
        _ => Kind::Leaf,
    };
    let children = match kind {
        Kind::Reject(label) => return Err(reject_unstorable(ctx, label)),
        Kind::Container(c) => c,
        Kind::Ordinary => {
            // `new Error(...)` / `new TypeError(...)` allocate `Ordinary` with
            // an error prototype (not `ObjectKind::Error`), so detect them by
            // walking the prototype chain to `Error.prototype`.
            if is_error_like(ctx.vm, proto) {
                return Err(reject_unstorable(ctx, "an Error"));
            }
            ctx.vm
                .get_object(id)
                .storage
                .iter_properties(&ctx.vm.shapes)
                .filter_map(|(_, val, attrs)| {
                    if !attrs.enumerable || attrs.is_accessor {
                        return None;
                    }
                    match val {
                        PropertyValue::Data(v) => Some(*v),
                        PropertyValue::Accessor { .. } => None,
                    }
                })
                .collect()
        }
        Kind::Leaf => return Ok(()),
    };
    for child in children {
        reject_non_json_storable(ctx, child, seen)?;
    }
    Ok(())
}

/// Build the `DataCloneError` for a cloneable-but-not-JSON-storable value.
fn reject_unstorable(ctx: &mut NativeContext<'_>, label: &str) -> VmError {
    dom_exc(
        ctx,
        "DataCloneError",
        format!(
            "{label} is not yet storable in IndexedDB \
             (structured-clone storage of this type is deferred)"
        ),
    )
}

/// Whether `proto`'s prototype chain reaches `Error.prototype` (or
/// `AggregateError.prototype`) — i.e. the object is an `Error` instance or a
/// subclass thereof.
fn is_error_like(vm: &VmInner, mut proto: Option<ObjectId>) -> bool {
    while let Some(p) = proto {
        if vm.error_prototype == Some(p) || vm.aggregate_error_prototype == Some(p) {
            return true;
        }
        proto = vm.get_object(p).prototype;
    }
    false
}

pub(crate) fn value_to_json(ctx: &mut NativeContext<'_>, val: JsValue) -> Result<String, VmError> {
    // §5.11: IDB clones via the structured clone algorithm, NOT JSON's
    // `toJSON` / silently-drop-functions semantics.  Structured-clone the value
    // first — this rejects a function or symbol ANYWHERE in the graph as a
    // `DataCloneError` (`JSON.stringify` would otherwise drop it silently,
    // `{ f() {} }` → `{}`, corrupting the stored value) and produces a plain
    // data graph.  Faithful binary storage of cloneable-but-not-JSON types
    // (Date / Map / ArrayBuffer) remains deferred to
    // `#11-idb-structured-clone-storage`.
    let cloned = super::super::structured_clone::clone_value(ctx.vm, val)?;
    // §5.11 v1 JSON storage gap: `clone_value` accepts cloneable-but-NOT-
    // JSON-representable types (RegExp / Error / ArrayBuffer / typed array /
    // DataView / Blob).  `JSON.stringify` silently coerces these to `{}` (no
    // enumerable own props) or an index-keyed object (typed arrays), so
    // `{ buf: new Uint8Array([1]) }` would be stored as `{ "buf": {"0":1} }`
    // and read back as a plain object — silent data corruption.  Reject the
    // whole value with `DataCloneError` until faithful binary / structured
    // storage lands (`#11-idb-structured-clone-storage`).  Primitive wrappers
    // (`Number`/`String`/`Boolean`) are fine — `JSON.stringify` unwraps them to
    // their primitive — and a `BigInt` wrapper loudly throws (→ `DataCloneError`
    // via the serialize-failure arm below), so neither needs rejection here.
    {
        let mut seen = std::collections::HashSet::new();
        reject_non_json_storable(ctx, cloned, &mut seen)?;
    }
    // Serialize THE CLONE, not the original: IDB stores the structured-cloned
    // value and must not invoke JSON serialization hooks — running
    // `JSON.stringify` on the original would re-run user `toJSON` methods /
    // accessors and could change what is persisted.  Root the clone on the VM
    // stack across the (allocating) serialization, then drop the scope.
    let serialized = {
        let mut frame = ctx.vm.push_stack_scope();
        frame.stack.push(cloned);
        let mut sub_ctx = NativeContext::new_call(&mut frame);
        super::super::super::natives_json::stringify_to_string(
            &mut sub_ctx,
            cloned,
            JsValue::Undefined,
            JsValue::Undefined,
        )
    };
    // §5.11 clone: ANY remaining serialization failure surfaces as
    // `DataCloneError`, never the raw `JSON.stringify` exception — a cyclic
    // structure throws `TypeError`, but for IDB the value simply cannot be
    // stored by this v1 JSON clone path, which is a clone failure.
    let Ok(serialized) = serialized else {
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
