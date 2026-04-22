//! `Blob` interface (File API §3, minimal Phase 2 form).
//!
//! `Blob` is a WebIDL interface rooted at `Object` — not an
//! `EventTarget`, not a `Node`.  Prototype chain:
//!
//! ```text
//! Blob instance (ObjectKind::Blob, payload-free)
//!   → Blob.prototype  (this module)
//!     → Object.prototype
//! ```
//!
//! ## Scope
//!
//! - `new Blob(blobParts?, options?)` — parts = `Array<BufferSource
//!   | Blob | USVString>`; options = `{type?: string, endings?:
//!   "transparent" | "native"}`.  `endings` is accepted but not
//!   acted upon (Phase 2 does not rewrite line endings).
//! - `.size` / `.type` IDL readonly attrs — authoritative internal
//!   slot (PR5a2 R7.1 lesson: `delete blob.size` must not break
//!   reads).
//! - `.slice(start?, end?, contentType?)` — range copy with
//!   optional type override.
//! - `.text()` / `.arrayBuffer()` — Promise-returning body reads.
//!   `.text()` uses `String::from_utf8_lossy` (TextDecoder comes
//!   with the next tranche).
//!
//! ## Deferred
//!
//! - `.stream()` — needs `ReadableStream`; throws `TypeError` for
//!   now.
//! - `File` subclass / `FileList`.
//! - Line-ending normalisation for `endings: "native"`.
//! - MIME type validation — Phase 2 accepts any ASCII string and
//!   ASCII-lowercases it when stored in [`BlobData::type_sid`]
//!   (matching Chromium / Firefox; the spec's full media-type
//!   parse lands with a later tranche).  Non-ASCII rejects with
//!   the empty type.

#![cfg(feature = "engine")]

use std::sync::Arc;

use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PromiseStatus, PropertyKey,
    PropertyStorage, PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};
use super::array_buffer::relative_index;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Per-`Blob` out-of-band state, keyed in
/// [`super::super::VmInner::blob_data`] by the instance's
/// `ObjectId`.  Bytes are an `Arc<[u8]>` so `.slice()` and
/// `.arrayBuffer()` can share the backing buffer across multiple
/// Blob / ArrayBuffer instances without copying the whole blob.
///
/// `type_sid` holds the lowercased MIME type (or the empty
/// `StringId` for `type === ""`).  The accessor returns it
/// verbatim — WHATWG §3 treats a missing/invalid type as the
/// empty string, which we represent with `well_known.empty`.
#[derive(Debug)]
pub(crate) struct BlobData {
    pub(crate) type_sid: StringId,
    pub(crate) bytes: Arc<[u8]>,
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `Blob.prototype`, install its accessor / method
    /// suite, and expose the `Blob` constructor on `globals`.
    ///
    /// Runs during `register_globals()` after `register_prototypes`.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — indicates a
    /// mis-ordered registration pass.
    pub(in crate::vm) fn register_blob_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_blob_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_blob_members(proto_id);
        self.blob_prototype = Some(proto_id);

        let ctor = self.create_constructable_function("Blob", native_blob_constructor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            PropertyAttrs::METHOD,
        );
        let name_sid = self.well_known.blob_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    fn install_blob_members(&mut self, proto_id: ObjectId) {
        // Snapshot StringIds up front so the subsequent `&mut self`
        // calls can't conflict with a live `&self.well_known`
        // borrow (E0502 — same pattern as ArrayBuffer / Headers).
        let accessors: [(StringId, NativeFn); 2] = [
            (self.well_known.size, native_blob_get_size as NativeFn),
            (self.well_known.event_type, native_blob_get_type as NativeFn),
        ];
        for (name_sid, getter) in accessors {
            let name = self.strings.get_utf8(name_sid);
            let gid = self.create_native_function(&format!("get {name}"), getter);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Accessor {
                    getter: Some(gid),
                    setter: None,
                },
                PropertyAttrs::WEBIDL_RO_ACCESSOR,
            );
        }

        let methods: [(StringId, NativeFn); 3] = [
            (self.well_known.slice, native_blob_slice as NativeFn),
            (self.well_known.text, native_blob_text as NativeFn),
            (
                self.well_known.array_buffer,
                native_blob_array_buffer as NativeFn,
            ),
        ];
        for (name_sid, func) in methods {
            let name = self.strings.get_utf8(name_sid);
            let fn_id = self.create_native_function(&name, func);
            self.define_shaped_property(
                proto_id,
                PropertyKey::String(name_sid),
                PropertyValue::Data(JsValue::Object(fn_id)),
                PropertyAttrs::METHOD,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Brand check + helpers
// ---------------------------------------------------------------------------

fn require_blob_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Blob.prototype.{method} called on non-Blob"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::Blob) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "Blob.prototype.{method} called on non-Blob"
        )))
    }
}

/// Length of the backing bytes, reading the authoritative slot.
pub(crate) fn blob_byte_length(vm: &VmInner, id: ObjectId) -> usize {
    vm.blob_data.get(&id).map_or(0, |d| d.bytes.len())
}

/// Return the backing `Arc<[u8]>` handle without cloning the
/// underlying bytes.  Used by the Body mixin when wrapping a Blob
/// into a Request / Response body (reference-counted share).
pub(crate) fn blob_bytes(vm: &VmInner, id: ObjectId) -> Arc<[u8]> {
    vm.blob_data
        .get(&id)
        .map_or_else(|| Arc::from(&[][..]), |d| Arc::clone(&d.bytes))
}

/// Return the blob's MIME type `StringId`, or `well_known.empty`
/// if no entry exists (defensive for a freshly allocated but
/// uninstalled instance).
pub(crate) fn blob_type(vm: &VmInner, id: ObjectId) -> StringId {
    vm.blob_data
        .get(&id)
        .map_or(vm.well_known.empty, |d| d.type_sid)
}

/// Allocate a `Blob` instance whose bytes are `bytes` and MIME
/// type is `type_sid`.  Used by `.slice()` and by the Body
/// mixin's `.blob()`.
pub(crate) fn create_blob_from_bytes(
    vm: &mut VmInner,
    bytes: Arc<[u8]>,
    type_sid: StringId,
) -> ObjectId {
    let proto = vm.blob_prototype;
    let id = vm.alloc_object(Object {
        kind: ObjectKind::Blob,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    vm.blob_data.insert(id, BlobData { type_sid, bytes });
    id
}

/// Synchronously resolve `promise` with `value`.  Idempotent —
/// relies on `settle_promise`'s `[[AlreadyResolved]]` guard.
///
/// **Invariant** (debug-checked): `promise` must have no
/// fulfill/reject reactions attached at call time.  All current
/// callers (Body mixin / Blob read methods / fetch) settle inline
/// on the same tick as `create_promise`, before the Promise
/// object is handed back to JS, so there is no opportunity for
/// user `.then()` to register a reaction.  If a future caller
/// leaks the Promise to JS before settling, the debug_assert
/// surfaces the misuse before reactions are silently dropped.
pub(super) fn resolve_promise_sync(vm: &mut VmInner, promise: ObjectId, value: JsValue) {
    // Directly mutate the promise state for the fulfilled case —
    // matches `natives_promise::fulfill_promise` minus the
    // microtask scheduling (all of our Body mixin callers invoke
    // this with a non-Promise value, so the spec's thenable
    // assimilation path does not apply).
    let obj = vm.get_object_mut(promise);
    if let ObjectKind::Promise(state) = &mut obj.kind {
        if matches!(state.status, PromiseStatus::Pending) && !state.already_resolved {
            debug_assert!(
                state.fulfill_reactions.is_empty() && state.reject_reactions.is_empty(),
                "resolve_promise_sync called on Promise with attached reactions — \
                 would silently drop them; use the full settle_promise path instead"
            );
            state.already_resolved = true;
            state.status = PromiseStatus::Fulfilled;
            state.result = value;
            state.fulfill_reactions.clear();
            state.reject_reactions.clear();
        }
    }
}

/// Synchronously reject `promise` with `reason`.  Same contract
/// as [`resolve_promise_sync`], including the no-pending-reactions
/// debug_assert.  Skips the unhandled-rejection queueing because
/// the Body mixin callers always attach a reaction via `await` /
/// `.then` / `.catch` in normal code paths — queueing would
/// produce a spurious warning for the common pattern
/// `await blob.text().catch(...)`.
pub(super) fn reject_promise_sync(vm: &mut VmInner, promise: ObjectId, reason: JsValue) {
    let obj = vm.get_object_mut(promise);
    if let ObjectKind::Promise(state) = &mut obj.kind {
        if matches!(state.status, PromiseStatus::Pending) && !state.already_resolved {
            debug_assert!(
                state.fulfill_reactions.is_empty() && state.reject_reactions.is_empty(),
                "reject_promise_sync called on Promise with attached reactions — \
                 would silently drop them; use the full settle_promise path instead"
            );
            state.already_resolved = true;
            state.status = PromiseStatus::Rejected;
            state.result = reason;
            state.fulfill_reactions.clear();
            state.reject_reactions.clear();
        }
    }
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

/// `new Blob(blobParts?, options?)` (File API §3.2).
fn native_blob_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'Blob': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };

    let parts_arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let options_arg = args.get(1).copied().unwrap_or(JsValue::Undefined);

    let bytes = match parts_arg {
        JsValue::Undefined => Arc::from(&[][..]),
        _ => collect_blob_parts_bytes(ctx, parts_arg)?,
    };
    let type_sid = parse_blob_options_type(ctx, options_arg)?;

    // Promote the pre-allocated Ordinary instance to Blob — do not
    // touch `prototype` so the `new.target.prototype` chain
    // installed by `do_new` survives (PR5a2 R7.2/R7.3 lesson).
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::Blob;
    ctx.vm
        .blob_data
        .insert(inst_id, BlobData { type_sid, bytes });
    Ok(JsValue::Object(inst_id))
}

/// Collect `blobParts` bytes into a single `Arc<[u8]>`.  Each
/// part is one of:
/// - Blob → bytes in order
/// - ArrayBuffer → bytes in order
/// - String → UTF-8 encoded bytes (no line-ending normalisation)
/// - Other → ToString → UTF-8
///
/// The spec requires `blobParts` to be iterable; Phase 2 only
/// supports plain Arrays (matches typical fetch usage).  Non-Array
/// objects produce `TypeError`.
fn collect_blob_parts_bytes(
    ctx: &mut NativeContext<'_>,
    parts: JsValue,
) -> Result<Arc<[u8]>, VmError> {
    let JsValue::Object(obj_id) = parts else {
        return Err(VmError::type_error(
            "Failed to construct 'Blob': The provided value cannot be converted to a sequence",
        ));
    };
    let elements: Vec<JsValue> = match &ctx.vm.get_object(obj_id).kind {
        ObjectKind::Array { elements } => elements.clone(),
        _ => {
            return Err(VmError::type_error(
                "Failed to construct 'Blob': The provided value cannot be converted to a sequence",
            ));
        }
    };
    let mut out: Vec<u8> = Vec::new();
    for part in elements {
        match part {
            JsValue::String(sid) => {
                let raw = ctx.vm.strings.get_utf8(sid);
                out.extend_from_slice(raw.as_bytes());
            }
            JsValue::Object(part_id) => match ctx.vm.get_object(part_id).kind {
                ObjectKind::ArrayBuffer => {
                    let bytes = super::array_buffer::array_buffer_bytes(ctx.vm, part_id);
                    out.extend_from_slice(&bytes);
                }
                ObjectKind::Blob => {
                    let bytes = blob_bytes(ctx.vm, part_id);
                    out.extend_from_slice(&bytes);
                }
                _ => {
                    // Fallback: stringify per WHATWG §3.2 step 2.3 (a
                    // non-BufferSource / non-Blob value becomes a
                    // USVString).
                    let sid = super::super::coerce::to_string(ctx.vm, part)?;
                    let raw = ctx.vm.strings.get_utf8(sid);
                    out.extend_from_slice(raw.as_bytes());
                }
            },
            _ => {
                let sid = super::super::coerce::to_string(ctx.vm, part)?;
                let raw = ctx.vm.strings.get_utf8(sid);
                out.extend_from_slice(raw.as_bytes());
            }
        }
    }
    Ok(Arc::from(out))
}

/// Parse the optional `options` dict's `type` member into a
/// lowercased `StringId`.  Spec §3.2 step 4 lower-cases the
/// provided MIME type after an ASCII validity check; Phase 2
/// lower-cases unconditionally (no validity check yet).
fn parse_blob_options_type(
    ctx: &mut NativeContext<'_>,
    options: JsValue,
) -> Result<StringId, VmError> {
    match options {
        JsValue::Undefined | JsValue::Null => Ok(ctx.vm.well_known.empty),
        JsValue::Object(opts_id) => {
            let type_key = PropertyKey::String(ctx.vm.well_known.event_type);
            let type_val = ctx.get_property_value(opts_id, type_key)?;
            match type_val {
                JsValue::Undefined => Ok(ctx.vm.well_known.empty),
                other => {
                    let sid = super::super::coerce::to_string(ctx.vm, other)?;
                    let raw = ctx.vm.strings.get_utf8(sid);
                    // ASCII-only validation — if any non-ASCII byte,
                    // fall back to empty (Chromium maps invalid types
                    // to empty string per spec §3.2 step 4.1).
                    let type_sid = if raw.bytes().all(|b| b.is_ascii()) {
                        let lower = raw.to_ascii_lowercase();
                        if lower == raw {
                            sid
                        } else {
                            ctx.vm.strings.intern(&lower)
                        }
                    } else {
                        ctx.vm.well_known.empty
                    };
                    Ok(type_sid)
                }
            }
        }
        _ => Err(VmError::type_error(
            "Failed to construct 'Blob': options must be an object",
        )),
    }
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

fn native_blob_get_size(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_blob_this(ctx, this, "size")?;
    #[allow(clippy::cast_precision_loss)]
    let size = blob_byte_length(ctx.vm, id) as f64;
    Ok(JsValue::Number(size))
}

fn native_blob_get_type(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_blob_this(ctx, this, "type")?;
    Ok(JsValue::String(blob_type(ctx.vm, id)))
}

// ---------------------------------------------------------------------------
// Methods
// ---------------------------------------------------------------------------

/// `Blob.prototype.slice(start?, end?, contentType?)` (File API §3.2.5).
fn native_blob_slice(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_blob_this(ctx, this, "slice")?;
    let len = blob_byte_length(ctx.vm, id);
    #[allow(clippy::cast_precision_loss)]
    let len_f = len as f64;

    let begin = match args.first().copied() {
        Some(JsValue::Undefined) | None => 0.0,
        Some(v) => super::super::coerce::to_number(ctx.vm, v)?,
    };
    let end = match args.get(1).copied() {
        Some(JsValue::Undefined) | None => len_f,
        Some(v) => super::super::coerce::to_number(ctx.vm, v)?,
    };
    let ct_sid = match args.get(2).copied() {
        Some(JsValue::Undefined) | None => ctx.vm.well_known.empty,
        Some(v) => {
            let sid = super::super::coerce::to_string(ctx.vm, v)?;
            let raw = ctx.vm.strings.get_utf8(sid);
            if raw.bytes().all(|b| b.is_ascii()) {
                let lower = raw.to_ascii_lowercase();
                if lower == raw {
                    sid
                } else {
                    ctx.vm.strings.intern(&lower)
                }
            } else {
                ctx.vm.well_known.empty
            }
        }
    };

    let start = relative_index(begin, len_f);
    let stop = relative_index(end, len_f);
    let final_len = stop.saturating_sub(start);

    let bytes: Arc<[u8]> = if final_len == 0 {
        Arc::from(&[][..])
    } else {
        let src = blob_bytes(ctx.vm, id);
        Arc::from(&src[start..stop])
    };
    let new_id = create_blob_from_bytes(ctx.vm, bytes, ct_sid);
    Ok(JsValue::Object(new_id))
}

/// `Blob.prototype.text()` → Promise<string> (File API §3.2.5).
/// Synchronously decodes the bytes as UTF-8 (lossy) and resolves
/// the returned Promise — spec calls for an async read, but our
/// Blob storage is already in memory so a microtask-free settle
/// matches observable behaviour for fulfilled promises.
fn native_blob_text(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_blob_this(ctx, this, "text")?;
    let promise = super::super::natives_promise::create_promise(ctx.vm);
    let bytes = blob_bytes(ctx.vm, id);
    let decoded = String::from_utf8_lossy(&bytes).into_owned();
    let sid = ctx.vm.strings.intern(&decoded);
    resolve_promise_sync(ctx.vm, promise, JsValue::String(sid));
    Ok(JsValue::Object(promise))
}

/// `Blob.prototype.arrayBuffer()` → Promise<ArrayBuffer>
/// (File API §3.2.5).
fn native_blob_array_buffer(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_blob_this(ctx, this, "arrayBuffer")?;
    let promise = super::super::natives_promise::create_promise(ctx.vm);
    let bytes = blob_bytes(ctx.vm, id);
    let buf_id = super::array_buffer::create_array_buffer_from_bytes(ctx.vm, bytes);
    resolve_promise_sync(ctx.vm, promise, JsValue::Object(buf_id));
    Ok(JsValue::Object(promise))
}

// `relative_index` is shared with ArrayBuffer.slice — see
// [`super::array_buffer::relative_index`].  Not hoisted into a
// dedicated helpers module because it only serves these two call
// sites at present.
