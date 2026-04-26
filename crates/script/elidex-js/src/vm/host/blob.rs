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
//! - `.stream()` — needs `ReadableStream`; not yet installed on
//!   `Blob.prototype` at all.  Calling `blob.stream()` currently
//!   surfaces the generic "method is not a function" TypeError
//!   from the property-access path, since no method is registered.
//!   Lands with the PR5-streams tranche.
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
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};
use super::array_buffer::relative_index;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Per-`Blob` out-of-band state, keyed in
/// [`super::super::VmInner::blob_data`] by the instance's
/// `ObjectId`.  Bytes are an `Arc<[u8]>` so whole-buffer ownership
/// can be shared cheaply (e.g. `.arrayBuffer()` hands a cloned
/// `Arc` to the new ArrayBuffer without copying).  `.slice()`
/// currently materialises a fresh `Arc<[u8]>` for the selected
/// sub-range rather than creating a shared offset/length view —
/// same trade-off as `ArrayBuffer.prototype.slice` (copy-on-slice
/// until the backing store is refactored to support shared
/// sub-range references).
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
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter,
                None,
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
            self.install_native_method(proto_id, name_sid, func, PropertyAttrs::METHOD);
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

/// Resolve `promise` with `value` via the standard
/// [`super::super::natives_promise::settle_promise`] path
/// (fulfill branch).  Kept as a thin helper because call sites
/// read more clearly with `resolve_promise_sync(vm, p, v)` than
/// with the 4-arg `settle_promise(vm, p, false, v)`.
///
/// The `_sync` name is historical: the earlier implementation
/// bypassed microtask scheduling / unhandled-rejection queueing
/// entirely, but that silently dropped rejections whose user
/// code lacked a `.catch` (Copilot R14/R15 findings).  Now we
/// delegate to the full settlement path — it is still
/// synchronous from the caller's perspective (the Promise
/// status flips immediately, no `await` needed by callers), but
/// reactions are properly enqueued and rejections participate in
/// unhandled-rejection tracking.
pub(super) fn resolve_promise_sync(vm: &mut VmInner, promise: ObjectId, value: JsValue) {
    let _ = super::super::natives_promise::settle_promise(vm, promise, false, value);
}

/// Reject `promise` with `reason` via
/// [`super::super::natives_promise::settle_promise`]'s reject
/// branch.  Symmetric with [`resolve_promise_sync`] — see that
/// helper for the `_sync` naming note.
///
/// Rejections that settle with no attached reaction are queued
/// on [`VmInner::pending_rejections`]; the end-of-drain scan
/// then either dispatches `unhandledrejection` (if a listener is
/// registered on the document) or logs to stderr, matching
/// WHATWG HTML §8.1.5.7.
pub(super) fn reject_promise_sync(vm: &mut VmInner, promise: ObjectId, reason: JsValue) {
    let _ = super::super::natives_promise::settle_promise(vm, promise, true, reason);
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

/// Convert a single `blobParts` element into bytes and append
/// them to `out`.  Spec §3.2 step 2 maps each part per the
/// `BlobPart = BufferSource or Blob or USVString` union:
/// - `Blob` instance → stored bytes
/// - `ArrayBuffer` → stored bytes
/// - everything else (including String and arbitrary JS values)
///   → `ToString` then UTF-8 bytes.
fn append_blob_part_bytes(
    ctx: &mut NativeContext<'_>,
    out: &mut Vec<u8>,
    part: JsValue,
) -> Result<(), VmError> {
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
            // TypedArray / DataView: WHATWG §3.2 BlobPart accepts
            // any BufferSource.  Append only the view's byte
            // range, not the full underlying buffer.
            ObjectKind::TypedArray {
                buffer_id,
                byte_offset,
                byte_length,
                ..
            }
            | ObjectKind::DataView {
                buffer_id,
                byte_offset,
                byte_length,
            } => {
                let backing = super::array_buffer::array_buffer_bytes(ctx.vm, buffer_id);
                let start = byte_offset as usize;
                let end = start + byte_length as usize;
                if let Some(slice) = backing.get(start..end) {
                    out.extend_from_slice(slice);
                }
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
    Ok(())
}

/// Collect `blobParts` bytes into a single `Arc<[u8]>`.
///
/// WebIDL types `blobParts` as `sequence<BlobPart>`, so conversion
/// runs through the iterator protocol — any value with a callable
/// `[Symbol.iterator]` is accepted (arrays, strings, user-defined
/// iterables, Sets, Maps).  Non-iterable values produce TypeError.
/// Abrupt completion (TypeError during `ToString` / `iter_next`
/// throwing) calls `IteratorClose` on custom iterables to let
/// `.return()` run cleanup, per ES §7.4.6 — same invariant we
/// added to `HeadersInit` parsing in R17.1 / R18.1 (R21.1).
fn collect_blob_parts_bytes(
    ctx: &mut NativeContext<'_>,
    parts: JsValue,
) -> Result<Arc<[u8]>, VmError> {
    // Array fast path: arrays have `@@iterator` on their prototype
    // and iterating would yield the same sequence, but reading
    // elements directly avoids the iterator protocol overhead.
    // Matches the analogous fast path in
    // `parse_headers_init_entries`.
    if let JsValue::Object(obj_id) = parts {
        if let ObjectKind::Array { elements } = &ctx.vm.get_object(obj_id).kind {
            let snapshot = elements.clone();
            let mut out: Vec<u8> = Vec::new();
            for part in snapshot {
                append_blob_part_bytes(ctx, &mut out, part)?;
            }
            return Ok(Arc::from(out));
        }
    }
    // Generic iterator protocol: consume whatever `[Symbol.iterator]`
    // yields.  `resolve_iterator` returns:
    // - `Some(iter)` for strings (String.prototype[@@iterator]) and
    //   any object whose `@@iterator` resolves to a callable.
    // - `None` for primitives without @@iterator (null, undefined,
    //   numbers, booleans) and for objects without an `@@iterator`
    //   property.  Either case fails WebIDL sequence conversion.
    let iter = match ctx.vm.resolve_iterator(parts)? {
        Some(iter @ JsValue::Object(_)) => iter,
        Some(_) => {
            return Err(VmError::type_error(
                "Failed to construct 'Blob': @@iterator must return an object",
            ));
        }
        None => {
            return Err(VmError::type_error(
                "Failed to construct 'Blob': The provided value cannot be converted to a sequence",
            ));
        }
    };
    let mut out: Vec<u8> = Vec::new();
    loop {
        // A throw from `iter_next` leaves the iterator already
        // closed (§7.4.6).  Propagate without calling `.return()`.
        let part = match ctx.vm.iter_next(iter)? {
            Some(p) => p,
            None => break,
        };
        // A throw during `append_blob_part_bytes` (e.g. `ToString`
        // on a Symbol operand) is an abrupt completion of the
        // for-of-like body; call `IteratorClose` before propagating.
        // `.return()` throwing takes precedence over the triggering
        // error (§7.4.6 step 6-7).
        if let Err(err) = append_blob_part_bytes(ctx, &mut out, part) {
            let close_err = ctx.vm.iter_close(iter).err();
            return Err(close_err.unwrap_or(err));
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
    // Keep `promise` rooted across the subsequent StringPool
    // intern — `alloc_object` inside intern() could otherwise
    // trigger GC between `create_promise` and
    // `resolve_promise_sync`, reclaiming the promise's slot
    // while it is only held in this Rust local.  The guard's
    // `Drop` pops the stack entry on every exit path.
    let mut g = ctx.vm.push_temp_root(JsValue::Object(promise));
    let bytes = blob_bytes(&g, id);
    let decoded = String::from_utf8_lossy(&bytes).into_owned();
    let sid = g.strings.intern(&decoded);
    resolve_promise_sync(&mut g, promise, JsValue::String(sid));
    drop(g);
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
    // Root `promise` across the subsequent `ArrayBuffer` alloc
    // (see `native_blob_text` for rationale).  The Blob instance
    // `id` is already reachable from JS (method receiver) so
    // `blob_bytes` stays safe without extra rooting.
    let mut g = ctx.vm.push_temp_root(JsValue::Object(promise));
    // BlobData stores `bytes` as `Arc<[u8]>` (per-spec immutable);
    // the new ArrayBuffer needs an owned `Vec<u8>` for `body_data`,
    // so snapshot at the pool boundary via `to_vec()`.
    let bytes = blob_bytes(&g, id).to_vec();
    let buf_id = super::array_buffer::create_array_buffer_from_bytes(&mut g, bytes);
    resolve_promise_sync(&mut g, promise, JsValue::Object(buf_id));
    drop(g);
    Ok(JsValue::Object(promise))
}

// `relative_index` is shared with ArrayBuffer.slice — see
// [`super::array_buffer::relative_index`].  Not hoisted into a
// dedicated helpers module because it only serves these two call
// sites at present.
