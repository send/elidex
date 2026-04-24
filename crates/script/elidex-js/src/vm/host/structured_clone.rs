//! `structuredClone(value, options?)` — WHATWG HTML §2.9
//! StructuredSerialize + StructuredDeserialize, fused into a single
//! recursive pass.
//!
//! Phase 2 is *same-realm* only — every produced object's prototype
//! is the target realm's corresponding intrinsic (which here equals
//! the source realm's, since there is exactly one realm).  Cross-
//! realm / worker clone lands with the cross-window postMessage
//! tranche (plan §Deferred #15 / PR5d).
//!
//! ## Algorithm
//!
//! - Primitives copy by value.
//! - Object values dispatch on [`ObjectKind`]; the supported set is
//!   narrow and deliberately enumerated: Ordinary / Array / RegExp /
//!   Error / primitive-wrappers / ArrayBuffer / Blob.  Everything
//!   else throws `DOMException("DataCloneError")` per spec §2.9 step
//!   27 ("otherwise throw DataCloneError").
//! - A cycle memo (source `ObjectId` → cloned `ObjectId`) makes
//!   `a.self = a` round-trip and shared references observe the same
//!   cloned identity on the output side.  **Memo insert happens
//!   before the recursive content walk** so self-references resolve
//!   against the just-allocated placeholder rather than re-cloning.
//!
//! ## GC contract
//!
//! [`clone_value`] runs under the native-call invariant
//! (`vm.gc_enabled == false`) — intermediate allocations cannot be
//! swept mid-clone and therefore do not need temp-root protection.
//! A `debug_assert!` captures the precondition; when the GC safety
//! token refactor lands (see `project_gc_safety_token.md`), the
//! assert becomes a type-level proof.
//!
//! ## Deferred types
//!
//! - Date / Map / Set — VM not yet implemented (§Deferred #10).
//! - TypedArray / DataView — PR5-typed-array (§Deferred #11).
//! - File / FileList — §Deferred #12.
//! - ImageData / ImageBitmap — §Deferred #13.
//! - Transferable objects (MessagePort / OffscreenCanvas) — §Deferred
//!   #14.  The `options.transfer` slot is therefore restricted to
//!   `undefined` / `[]`; a non-empty transfer list throws
//!   `DataCloneError` until the Phase 3 wiring lands.

#![cfg(feature = "engine")]

use std::collections::HashMap;
use std::sync::Arc;

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::VmInner;
use super::blob::BlobData;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Structured-clone `input` under the WHATWG HTML §2.9 StructuredSerialize
/// + StructuredDeserialize fused algorithm.
///
/// Returns the cloned value on success, or a
/// `DOMException("DataCloneError")` when the input contains an
/// unsupported type.  Primitive inputs pass through unchanged.
///
/// # Panics (debug builds)
///
/// Panics if `vm.gc_enabled` is `true` — clone allocates without
/// temp-rooting intermediates, which is safe only while the GC is
/// disabled (the guarantee held by every native-function entry).
pub(crate) fn clone_value(vm: &mut VmInner, input: JsValue) -> Result<JsValue, VmError> {
    debug_assert!(
        !vm.gc_enabled,
        "clone_value requires gc_enabled == false (native-call invariant); allocations here are not temp-rooted",
    );
    let mut memo: HashMap<ObjectId, ObjectId> = HashMap::new();
    clone_recursive(vm, input, &mut memo)
}

// ---------------------------------------------------------------------------
// Recursive worker
// ---------------------------------------------------------------------------

fn clone_recursive(
    vm: &mut VmInner,
    input: JsValue,
    memo: &mut HashMap<ObjectId, ObjectId>,
) -> Result<JsValue, VmError> {
    let src_id = match input {
        // Primitives: StringId / BigIntId / SymbolId handles are
        // pool-permanent, so copying the handle is copy-by-value at
        // the spec level as well.
        JsValue::Empty
        | JsValue::Undefined
        | JsValue::Null
        | JsValue::Boolean(_)
        | JsValue::Number(_)
        | JsValue::String(_)
        | JsValue::BigInt(_) => return Ok(input),
        // Bare Symbol primitives are explicitly unclonable (spec §2.9
        // "if Type(input) is Symbol, throw DataCloneError").  Symbol
        // *wrappers* (`Object(Symbol())`) are also rejected below
        // under [`ObjectKind::SymbolWrapper`].
        JsValue::Symbol(_) => return Err(data_clone_error(vm, "Symbol")),
        JsValue::Object(id) => id,
    };

    if let Some(&cloned) = memo.get(&src_id) {
        return Ok(JsValue::Object(cloned));
    }

    // Snapshot the fields we need from the source object up-front so
    // the subsequent `alloc_object` (which takes `&mut vm`) does not
    // run while we still hold a `&Object` borrow.
    let src_obj = vm.get_object(src_id);
    let src_proto = src_obj.prototype;
    let kind_kind = classify(&src_obj.kind);

    match kind_kind {
        // An `Ordinary` object whose prototype is `error_prototype`
        // (or any Error-subclass prototype that chains to it) is a
        // user-constructed `new Error(...) / new TypeError(...)` —
        // the elidex ctors allocate Ordinary with `ObjectKind::Ordinary`
        // rather than `ObjectKind::Error`, so we detect them by
        // prototype identity.  Routing through `clone_error`
        // preserves the proto chain so `instanceof TypeError` holds
        // on the clone; a plain `{}` still falls through to
        // `clone_ordinary`.
        CloneKind::Ordinary if is_error_like_proto(vm, src_proto) => {
            clone_error(vm, src_id, src_proto, memo)
        }
        CloneKind::Ordinary => clone_ordinary(vm, src_id, memo),
        CloneKind::Array => clone_array(vm, src_id, memo),
        CloneKind::RegExp => clone_regexp(vm, src_id),
        CloneKind::Error => clone_error(vm, src_id, src_proto, memo),
        CloneKind::NumberWrapper(n) => Ok(JsValue::Object(alloc_wrapper(
            vm,
            ObjectKind::NumberWrapper(n),
            vm.number_prototype,
        ))),
        CloneKind::StringWrapper(sid) => Ok(JsValue::Object(alloc_wrapper(
            vm,
            ObjectKind::StringWrapper(sid),
            vm.string_prototype,
        ))),
        CloneKind::BooleanWrapper(b) => Ok(JsValue::Object(alloc_wrapper(
            vm,
            ObjectKind::BooleanWrapper(b),
            vm.boolean_prototype,
        ))),
        CloneKind::BigIntWrapper(id) => Ok(JsValue::Object(alloc_wrapper(
            vm,
            ObjectKind::BigIntWrapper(id),
            vm.bigint_prototype,
        ))),
        CloneKind::ArrayBuffer => Ok(JsValue::Object(clone_array_buffer(vm, src_id))),
        CloneKind::Blob => Ok(JsValue::Object(clone_blob(vm, src_id))),
        CloneKind::Unclonable(label) => Err(data_clone_error(vm, label)),
    }
}

// ---------------------------------------------------------------------------
// Classification
// ---------------------------------------------------------------------------

/// Compact intermediate discriminator — lets the dispatch read a
/// single owned value instead of continuing to hold `&Object` borrows
/// while allocating.  Variants that carry data copy the primitive
/// payload out of the source kind so the clone path never needs to
/// re-borrow the source.
enum CloneKind {
    Ordinary,
    Array,
    RegExp,
    Error,
    NumberWrapper(f64),
    StringWrapper(super::super::value::StringId),
    BooleanWrapper(bool),
    BigIntWrapper(super::super::value::BigIntId),
    ArrayBuffer,
    Blob,
    Unclonable(&'static str),
}

fn classify(kind: &ObjectKind) -> CloneKind {
    match kind {
        ObjectKind::Ordinary => CloneKind::Ordinary,
        ObjectKind::Array { .. } => CloneKind::Array,
        ObjectKind::RegExp { .. } => CloneKind::RegExp,
        ObjectKind::Error { .. } => CloneKind::Error,
        ObjectKind::NumberWrapper(n) => CloneKind::NumberWrapper(*n),
        ObjectKind::StringWrapper(s) => CloneKind::StringWrapper(*s),
        ObjectKind::BooleanWrapper(b) => CloneKind::BooleanWrapper(*b),
        ObjectKind::BigIntWrapper(id) => CloneKind::BigIntWrapper(*id),
        ObjectKind::ArrayBuffer => CloneKind::ArrayBuffer,
        ObjectKind::Blob => CloneKind::Blob,
        // Explicit unclonables — labels feed the DOMException message
        // so authors see which type tripped the error.
        ObjectKind::Function(_)
        | ObjectKind::NativeFunction(_)
        | ObjectKind::BoundFunction { .. } => CloneKind::Unclonable("Function"),
        ObjectKind::SymbolWrapper(_) => CloneKind::Unclonable("Symbol"),
        ObjectKind::Promise(_)
        | ObjectKind::PromiseResolver { .. }
        | ObjectKind::PromiseCombinatorState(_)
        | ObjectKind::PromiseCombinatorStep(_)
        | ObjectKind::PromiseFinallyStep { .. }
        | ObjectKind::AsyncDriverStep { .. } => CloneKind::Unclonable("Promise"),
        ObjectKind::Generator(_) => CloneKind::Unclonable("Generator"),
        ObjectKind::HostObject { .. } => CloneKind::Unclonable("HostObject"),
        ObjectKind::Event { .. } => CloneKind::Unclonable("Event"),
        ObjectKind::ForInIterator(_)
        | ObjectKind::ArrayIterator(_)
        | ObjectKind::StringIterator(_) => CloneKind::Unclonable("Iterator"),
        ObjectKind::Arguments { .. } => CloneKind::Unclonable("Arguments"),
        ObjectKind::AbortSignal | ObjectKind::AbortController { .. } => {
            CloneKind::Unclonable("AbortSignal")
        }
        ObjectKind::Headers => CloneKind::Unclonable("Headers"),
        ObjectKind::Request => CloneKind::Unclonable("Request"),
        ObjectKind::Response => CloneKind::Unclonable("Response"),
        ObjectKind::HtmlCollection => CloneKind::Unclonable("HTMLCollection"),
        ObjectKind::NodeList => CloneKind::Unclonable("NodeList"),
        ObjectKind::NamedNodeMap => CloneKind::Unclonable("NamedNodeMap"),
        ObjectKind::Attr => CloneKind::Unclonable("Attr"),
        // TypedArray / DataView clone support lands in PR5-typed-array
        // §C6 (along with `clone_array_buffer` memo-threading refactor
        // so shared-buffer identity survives the walk).  The C1
        // scaffolding commit keeps them Unclonable so the compiler-
        // enforced exhaustive match passes while the ctor / proto
        // registration is still in-flight.
        ObjectKind::TypedArray { .. } => CloneKind::Unclonable("TypedArray"),
        ObjectKind::DataView { .. } => CloneKind::Unclonable("DataView"),
    }
}

// ---------------------------------------------------------------------------
// Per-kind clone helpers
// ---------------------------------------------------------------------------

/// Clone a plain `{}` — copy every own enumerable data property,
/// recursing into the value.  Accessor properties are not exposed by
/// structuredClone (spec §2.9: only data properties are walked); any
/// accessor is silently skipped, matching browsers.
fn clone_ordinary(
    vm: &mut VmInner,
    src: ObjectId,
    memo: &mut HashMap<ObjectId, ObjectId>,
) -> Result<JsValue, VmError> {
    let new_id = vm.alloc_object(Object {
        kind: ObjectKind::Ordinary,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.object_prototype,
        extensible: true,
    });
    memo.insert(src, new_id);

    // Snapshot the (key, value) list before recursing — `clone_recursive`
    // takes `&mut vm` and may invalidate any `&Object` borrow.
    let entries: Vec<(PropertyKey, JsValue)> = vm
        .get_object(src)
        .storage
        .iter_properties(&vm.shapes)
        .filter_map(|(key, val, attrs)| {
            if !attrs.enumerable || attrs.is_accessor {
                return None;
            }
            match val {
                PropertyValue::Data(v) => Some((key, *v)),
                PropertyValue::Accessor { .. } => None,
            }
        })
        .collect();

    for (key, v) in entries {
        let cloned = clone_recursive(vm, v, memo)?;
        vm.define_shaped_property(
            new_id,
            key,
            PropertyValue::Data(cloned),
            PropertyAttrs::DATA,
        );
    }
    Ok(JsValue::Object(new_id))
}

/// Clone an Array — recurse each element (hole preservation via
/// [`JsValue::Empty`]) and drop any own `length` / indexed data
/// properties onto the dense storage.  Non-index enumerable own data
/// properties (e.g. `arr.foo = 1`) are copied onto the result with
/// [`PropertyAttrs::DATA`].
fn clone_array(
    vm: &mut VmInner,
    src: ObjectId,
    memo: &mut HashMap<ObjectId, ObjectId>,
) -> Result<JsValue, VmError> {
    let src_elements: Vec<JsValue> = match &vm.get_object(src).kind {
        ObjectKind::Array { elements } => elements.clone(),
        _ => unreachable!("classify dispatched non-Array to clone_array"),
    };
    let new_id = vm.alloc_object(Object {
        kind: ObjectKind::Array {
            elements: Vec::with_capacity(src_elements.len()),
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: vm.array_prototype,
        extensible: true,
    });
    memo.insert(src, new_id);

    let mut cloned_elements = Vec::with_capacity(src_elements.len());
    for v in src_elements {
        // Holes (Empty) carry over literally — cloning a sparse array
        // must preserve hole positions, not densify them.
        if v.is_empty() {
            cloned_elements.push(JsValue::Empty);
        } else {
            cloned_elements.push(clone_recursive(vm, v, memo)?);
        }
    }
    if let ObjectKind::Array { elements } = &mut vm.get_object_mut(new_id).kind {
        *elements = cloned_elements;
    }

    // Named own enumerable data properties (e.g. `arr.tag = "x"`).
    let extra: Vec<(PropertyKey, JsValue)> = vm
        .get_object(src)
        .storage
        .iter_properties(&vm.shapes)
        .filter_map(|(key, val, attrs)| {
            if !attrs.enumerable || attrs.is_accessor {
                return None;
            }
            match val {
                PropertyValue::Data(v) => Some((key, *v)),
                PropertyValue::Accessor { .. } => None,
            }
        })
        .collect();
    for (key, v) in extra {
        let cloned = clone_recursive(vm, v, memo)?;
        vm.define_shaped_property(
            new_id,
            key,
            PropertyValue::Data(cloned),
            PropertyAttrs::DATA,
        );
    }
    Ok(JsValue::Object(new_id))
}

/// Clone a RegExp — re-compile the source pattern + flags.  Per
/// §2.9 step 15, `lastIndex` is **not** carried over; fresh RegExp
/// instances start at 0.
fn clone_regexp(vm: &mut VmInner, src: ObjectId) -> Result<JsValue, VmError> {
    let (pattern, flags) = match &vm.get_object(src).kind {
        ObjectKind::RegExp { pattern, flags, .. } => (*pattern, *flags),
        _ => unreachable!("classify dispatched non-RegExp to clone_regexp"),
    };
    let pattern_str = vm.strings.get_utf8(pattern);
    let flags_str = vm.strings.get_utf8(flags);
    let regex_flags = super::super::dispatch_helpers::regress_flags_from_str(&flags_str);
    // A valid source RegExp's pattern + flags re-compile unchanged;
    // the error branch is defensive and never observed in practice.
    let compiled = regress::Regex::with_flags(&pattern_str, regex_flags).map_err(|e| {
        VmError::type_error(format!("structuredClone: failed to re-compile RegExp: {e}"))
    })?;
    let proto = vm.regexp_prototype;
    let new_id = vm.alloc_object(Object {
        kind: ObjectKind::RegExp {
            pattern,
            flags,
            compiled: Box::new(compiled),
        },
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    // `source` / `flags` / `lastIndex` are own data properties on
    // RegExp instances (§21.2.5.10 / §21.2.5.3 / §21.2.5.3).  The
    // PushConst path installs them at ctor time with the same
    // attrs; we mirror that layout here so the cloned RegExp's
    // shape matches a freshly-constructed one.  `lastIndex` resets
    // to 0 per §2.9 step 15.
    vm.define_shaped_property(
        new_id,
        PropertyKey::String(vm.well_known.source),
        PropertyValue::Data(JsValue::String(pattern)),
        PropertyAttrs::BUILTIN,
    );
    vm.define_shaped_property(
        new_id,
        PropertyKey::String(vm.well_known.flags),
        PropertyValue::Data(JsValue::String(flags)),
        PropertyAttrs::BUILTIN,
    );
    vm.define_shaped_property(
        new_id,
        PropertyKey::String(vm.well_known.last_index),
        PropertyValue::Data(JsValue::Number(0.0)),
        PropertyAttrs::WRITABLE_HIDDEN,
    );
    Ok(JsValue::Object(new_id))
}

/// Clone an Error — preserve both the `ObjectKind::Error { name }`
/// payload and the source `.prototype` (so `new TypeError(...)`
/// round-trips as a TypeError).  `stack` is non-standard and is
/// deliberately dropped.
///
/// The `memo` is threaded the same way as `clone_ordinary` /
/// `clone_array`: the `src → new_id` entry is installed **before**
/// recursive value walks, so an Error participating in a cycle
/// (`err.cause === err`, or `obj.err = err; err.obj = obj`) resolves
/// to the just-allocated placeholder rather than re-cloning.  Own
/// data property values (notably `cause`) are recursively cloned so
/// the clone graph shares no object references with the source.
fn clone_error(
    vm: &mut VmInner,
    src: ObjectId,
    src_proto: Option<ObjectId>,
    memo: &mut HashMap<ObjectId, ObjectId>,
) -> Result<JsValue, VmError> {
    // `new TypeError(...)` allocates an Ordinary with
    // `error_prototype`; the VM-internal thrown path allocates
    // `ObjectKind::Error { name }`.  The clone carries the kind
    // through so JSON / toString / typeof agree with the source.
    let kind = match &vm.get_object(src).kind {
        ObjectKind::Error { name } => ObjectKind::Error { name: *name },
        ObjectKind::Ordinary => ObjectKind::Ordinary,
        _ => unreachable!("classify dispatched non-Error to clone_error"),
    };
    let new_id = vm.alloc_object(Object {
        kind,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: src_proto.or(vm.error_prototype),
        extensible: true,
    });
    // Install memo BEFORE recursive walks so self-referencing Errors
    // (`err.cause = err`) resolve to `new_id` instead of re-cloning.
    memo.insert(src, new_id);
    // §19.5.1.1 replicates: copy own `.name` and `.message` data
    // properties with METHOD attrs, matching the ctor.  Any other
    // own data properties (incl. `cause`) also copy through —
    // recursively cloned so object values do not leak references
    // back to the source graph.
    let entries: Vec<(PropertyKey, JsValue, PropertyAttrs)> = vm
        .get_object(src)
        .storage
        .iter_properties(&vm.shapes)
        .filter_map(|(key, val, attrs)| {
            if attrs.is_accessor {
                return None;
            }
            match val {
                PropertyValue::Data(v) => Some((key, *v, attrs)),
                PropertyValue::Accessor { .. } => None,
            }
        })
        .collect();
    let stack_sid = vm.strings.intern("stack");
    for (key, v, attrs) in entries {
        if matches!(key, PropertyKey::String(sid) if sid == stack_sid) {
            continue;
        }
        let cloned = clone_recursive(vm, v, memo)?;
        vm.define_shaped_property(new_id, key, PropertyValue::Data(cloned), attrs);
    }
    Ok(JsValue::Object(new_id))
}

/// Returns `true` if `proto` is the Error-family prototype chain
/// (Error.prototype or AggregateError.prototype).  In elidex every
/// subclass ctor (TypeError / RangeError / …) shares
/// `error_prototype`, so a user `new TypeError(...)` instance's
/// prototype is exactly that value.
fn is_error_like_proto(vm: &VmInner, proto: Option<ObjectId>) -> bool {
    match proto {
        Some(p) => vm.error_prototype == Some(p) || vm.aggregate_error_prototype == Some(p),
        None => false,
    }
}

fn alloc_wrapper(vm: &mut VmInner, kind: ObjectKind, proto: Option<ObjectId>) -> ObjectId {
    vm.alloc_object(Object {
        kind,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    })
}

/// Deep-copy an ArrayBuffer.  The backing bytes are freshly allocated
/// (no `Arc::clone` sharing) so mutations through one buffer cannot
/// be observed through the other — the defining contract of
/// StructuredSerialize for transferable ArrayBuffers when no
/// `transfer` list is present (spec §2.9 step 12).
fn clone_array_buffer(vm: &mut VmInner, src: ObjectId) -> ObjectId {
    let src_bytes: Arc<[u8]> = vm
        .body_data
        .get(&src)
        .cloned()
        .unwrap_or_else(|| Arc::from(&[][..]));
    // `Arc::<[u8]>::from(&[u8])` performs a single allocation + memcpy
    // straight into the Arc payload — independent memory, no shared
    // refcount with the source (StructuredSerialize §2.9 step 12).
    let new_bytes: Arc<[u8]> = Arc::<[u8]>::from(&src_bytes[..]);
    super::array_buffer::create_array_buffer_from_bytes(vm, new_bytes)
}

/// Deep-copy a Blob.  Bytes are `to_vec()`-copied for the same
/// independence guarantee as ArrayBuffer; `type` is a pool-interned
/// `StringId` and so survives the clone without re-interning.
fn clone_blob(vm: &mut VmInner, src: ObjectId) -> ObjectId {
    let (bytes, type_sid) = match vm.blob_data.get(&src) {
        Some(BlobData { bytes, type_sid }) => (Arc::clone(bytes), *type_sid),
        None => (Arc::from(&[][..]), vm.well_known.empty),
    };
    let new_bytes: Arc<[u8]> = Arc::<[u8]>::from(&bytes[..]);
    super::blob::create_blob_from_bytes(vm, new_bytes, type_sid)
}

// ---------------------------------------------------------------------------
// Error helper
// ---------------------------------------------------------------------------

fn data_clone_error(vm: &VmInner, label: &str) -> VmError {
    VmError::dom_exception(
        vm.well_known.dom_exc_data_clone_error,
        format!("{label} could not be cloned."),
    )
}

// ---------------------------------------------------------------------------
// Global registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Install `window.structuredClone` — runs during
    /// `register_globals()` after `register_blob_global()` /
    /// `register_array_buffer_global()` so the wrapper-allocation
    /// helpers have their prototypes in place.
    pub(in crate::vm) fn register_structured_clone_global(&mut self) {
        let name = "structuredClone";
        let fn_id = self.create_native_function(name, native_structured_clone);
        let name_sid = self.strings.intern(name);
        self.globals.insert(name_sid, JsValue::Object(fn_id));
    }
}

/// `structuredClone(value, options?)` — spec §2.9.
///
/// Binding-level checks: missing `value` → sync `TypeError`,
/// non-empty `options.transfer` → sync `DataCloneError`
/// (transferable objects are deferred to Phase 3; a non-empty list
/// is therefore never satisfiable).
///
/// All post-binding failures (unsupported `ObjectKind` inside
/// `value`) throw `DOMException("DataCloneError")` synchronously —
/// structuredClone is itself synchronous (no Promise wrapping), so
/// the error surface is the standard throw path.
pub(super) fn native_structured_clone(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let Some(&value) = args.first() else {
        return Err(VmError::type_error(
            "Failed to execute 'structuredClone' on 'Window': 1 argument required, but only 0 present.",
        ));
    };

    // `options.transfer` validation — `undefined` / missing is the
    // only pass through path in Phase 2.  Empty array is also OK.
    // Any other truthy object triggers DataCloneError (transferable
    // semantics not yet wired).
    let options = args.get(1).copied().unwrap_or(JsValue::Undefined);
    validate_transfer(ctx, options)?;

    clone_value(ctx.vm, value)
}

fn validate_transfer(ctx: &mut NativeContext<'_>, options: JsValue) -> Result<(), VmError> {
    match options {
        JsValue::Undefined | JsValue::Null => Ok(()),
        JsValue::Object(opts_id) => {
            // WebIDL dictionary conversion: ordinary `Get` walks
            // proto chain and fires getters — a `storage.get` direct
            // read would silently skip accessor-defined / inherited
            // `transfer` entries.
            let transfer_key = PropertyKey::String(ctx.vm.strings.intern("transfer"));
            let transfer_val = ctx.vm.get_property_value(opts_id, transfer_key)?;
            ensure_empty_transfer_list(
                ctx,
                transfer_val,
                "Failed to execute 'structuredClone' on 'Window'",
            )
        }
        _ => Err(VmError::type_error(
            "Failed to execute 'structuredClone' on 'Window': The provided value is not of type 'StructuredSerializeOptions'.",
        )),
    }
}

/// Validate a transfer-list argument as a WebIDL
/// `sequence<object>` that resolves to an empty list.
///
/// Phase 2 restriction: real transferable objects are not yet
/// supported, so a non-empty list throws `DataCloneError`.  A
/// non-iterable Object (e.g. a plain `{}`, or an Object whose
/// `@@iterator` is undefined) throws `TypeError` per WebIDL §3.2.27
/// step 2-3, mirroring the TypeError thrown for non-Object
/// primitives.  Shared by `structuredClone`'s
/// `StructuredSerializeOptions.transfer` and
/// `window.postMessage`'s transfer argument (legacy and dict form).
pub(super) fn ensure_empty_transfer_list(
    ctx: &mut NativeContext<'_>,
    transfer: JsValue,
    err_prefix: &str,
) -> Result<(), VmError> {
    match transfer {
        JsValue::Undefined | JsValue::Null => Ok(()),
        JsValue::Object(obj_id) => {
            // Fast path: Array with empty elements (the common case).
            if let ObjectKind::Array { elements } = &ctx.vm.get_object(obj_id).kind {
                if elements.is_empty() {
                    return Ok(());
                }
                return Err(VmError::dom_exception(
                    ctx.vm.well_known.dom_exc_data_clone_error,
                    format!("{err_prefix}: Transferable objects are not yet supported."),
                ));
            }
            // Non-Array object → probe `@@iterator` (WebIDL §3.2.27
            // step 2-3).  Missing `@@iterator` → TypeError, not
            // DataCloneError.
            let iter_key = PropertyKey::Symbol(ctx.vm.well_known_symbols.iterator);
            let iter_method = ctx.vm.get_property_value(obj_id, iter_key)?;
            let iter_fn = match iter_method {
                JsValue::Undefined | JsValue::Null => {
                    return Err(VmError::type_error(format!(
                        "{err_prefix}: transfer is not iterable"
                    )));
                }
                JsValue::Object(id) if ctx.vm.get_object(id).kind.is_callable() => iter_method,
                _ => {
                    return Err(VmError::type_error(format!(
                        "{err_prefix}: @@iterator is not callable"
                    )));
                }
            };
            // Invoke the iterator and probe just the first `next()`.
            // Empty iterable → OK; non-empty → DataCloneError (Phase
            // 2 does not support transferables).
            let iter = ctx.vm.call_value(iter_fn, transfer, &[])?;
            if !matches!(iter, JsValue::Object(_)) {
                return Err(VmError::type_error(format!(
                    "{err_prefix}: @@iterator must return an object"
                )));
            }
            // A throw from `iter_next` itself means the iterator's
            // own `.next()` raised — per ES §7.4.5 / §7.4.7
            // (IteratorStep / IteratorStepValue), the spec sets
            // `iteratorRecord.[[Done]] = true` and propagates the
            // completion WITHOUT invoking `IteratorClose`.  WebIDL
            // §3.2.27 "Creating a sequence from iterable" inherits
            // that behaviour via `?`.  Same convention documented
            // in `headers.rs` (`parse_init`) and `blob.rs`
            // (`blob_ctor_parts`); propagating the error via `?`
            // is spec-compliant.
            match ctx.vm.iter_next(iter)? {
                None => Ok(()),
                Some(_) => {
                    // Non-empty iteration is an abrupt completion
                    // from OUR loop body (not the iterator
                    // itself), so `IteratorClose` must run before
                    // surfacing the Phase 2 limitation (§7.4.8
                    // IteratorClose).  A `.return()` throw takes
                    // precedence over the triggering abrupt.
                    if let Some(close_err) = ctx.vm.iter_close(iter).err() {
                        return Err(close_err);
                    }
                    Err(VmError::dom_exception(
                        ctx.vm.well_known.dom_exc_data_clone_error,
                        format!("{err_prefix}: Transferable objects are not yet supported."),
                    ))
                }
            }
        }
        _ => Err(VmError::type_error(format!(
            "{err_prefix}: transfer is not iterable"
        ))),
    }
}
