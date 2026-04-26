//! `ArrayBuffer` interface (ES2020 §24.1, minimal Phase 2 form).
//!
//! `ArrayBuffer` is an ES built-in whose prototype chain is simply:
//!
//! ```text
//! ArrayBuffer instance (ObjectKind::ArrayBuffer, payload-free)
//!   → ArrayBuffer.prototype  (this module)
//!     → Object.prototype
//! ```
//!
//! ## Scope
//!
//! - `new ArrayBuffer(length)` — `length` coerced via a `ToIndex`-shaped
//!   path (negative / non-finite integers reject with `RangeError`).
//! - `.byteLength` IDL readonly attr (authoritative internal slot).
//! - `.slice(begin?, end?)` — allocates a fresh ArrayBuffer whose
//!   bytes are a range copy of the receiver's backing buffer.
//!   Range resolution matches `Array.prototype.slice` for negative
//!   indices and out-of-range clamping.
//!
//! ## Storage
//!
//! The backing bytes live in [`super::super::VmInner::body_data`]
//! (shared with `Request` / `Response` / Blob body reads) — not a
//! private `array_buffer_data` map.  This keeps GC sweep pruning
//! unified: the existing `body_data.retain(|id, _| live)` in
//! `gc.rs` already drops dead entries, so `ArrayBuffer` adds no
//! new post-sweep logic.
//!
//! ## Deferred
//!
//! - `SharedArrayBuffer` / detached state / `resizable` ctor
//!   option (ES2024).
//! - `.transfer()` / `.transferToFixedLength()` / `.resize()`.
//! - TypedArray views (`Uint8Array` / `DataView` / …) — next tranche.
//! - `ArrayBuffer.isView` static (pointless without any views).

#![cfg(feature = "engine")]

use std::sync::Arc;

use super::super::coerce;
use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `ArrayBuffer.prototype`, install the accessor /
    /// method suite, and expose the `ArrayBuffer` constructor on
    /// `globals`.
    ///
    /// Runs during `register_globals()` after `register_prototypes`
    /// populates `object_prototype`.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — indicates a
    /// mis-ordered registration pass.
    pub(in crate::vm) fn register_array_buffer_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_array_buffer_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.install_array_buffer_members(proto_id);
        self.array_buffer_prototype = Some(proto_id);

        let ctor =
            self.create_constructable_function("ArrayBuffer", native_array_buffer_constructor);
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
        let name_sid = self.well_known.array_buffer_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    /// Install `ArrayBuffer.isView` static on the `ArrayBuffer`
    /// constructor (ES §25.1.4.3).  Returns `true` for
    /// `ObjectKind::TypedArray` / `ObjectKind::DataView` instances,
    /// `false` for everything else (including plain ArrayBuffers —
    /// the spec is deliberate: `isView(ab)` is `false`).  No throw,
    /// no coerce.
    ///
    /// Must run **after** `register_typed_array_prototype_global`
    /// and `register_data_view_global` — the function body's
    /// brand check pattern-matches both variants, which exist
    /// unconditionally (inline in `ObjectKind`), but callers of
    /// this getter expect to observe real TypedArray / DataView
    /// instances in the runtime.  Registration-order independence
    /// is enforced by looking up the `ArrayBuffer` ctor via
    /// `globals` rather than an explicit argument.
    pub(in crate::vm) fn install_array_buffer_is_view(&mut self) {
        let Some(JsValue::Object(ctor)) = self
            .globals
            .get(&self.well_known.array_buffer_global)
            .copied()
        else {
            return;
        };
        let is_view_sid = self.strings.intern("isView");
        let is_view_fn = self.create_native_function("isView", native_array_buffer_is_view);
        self.define_shaped_property(
            ctor,
            PropertyKey::String(is_view_sid),
            PropertyValue::Data(JsValue::Object(is_view_fn)),
            PropertyAttrs::METHOD,
        );
    }

    fn install_array_buffer_members(&mut self, proto_id: ObjectId) {
        // Snapshot StringIds up front so the subsequent `&mut self`
        // calls don't conflict with a live `&self.well_known`
        // borrow (E0502 — same pattern as Request/Response
        // accessors).
        let byte_length_sid = self.well_known.byte_length;
        let getter = self.create_native_function(
            "get byteLength",
            native_array_buffer_get_byte_length as NativeFn,
        );
        self.define_shaped_property(
            proto_id,
            PropertyKey::String(byte_length_sid),
            PropertyValue::Accessor {
                getter: Some(getter),
                setter: None,
            },
            PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        let slice_sid = self.well_known.slice;
        let slice_fn = self.create_native_function("slice", native_array_buffer_slice as NativeFn);
        self.define_shaped_property(
            proto_id,
            PropertyKey::String(slice_sid),
            PropertyValue::Data(JsValue::Object(slice_fn)),
            PropertyAttrs::METHOD,
        );
    }
}

// ---------------------------------------------------------------------------
// Brand check + helpers
// ---------------------------------------------------------------------------

fn require_array_buffer_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "ArrayBuffer.prototype.{method} called on non-ArrayBuffer"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::ArrayBuffer) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "ArrayBuffer.prototype.{method} called on non-ArrayBuffer"
        )))
    }
}

/// Length of the backing bytes for an ArrayBuffer.  Missing map
/// entry ⇒ zero-length, which matches a freshly allocated but
/// uninstalled instance (defensive — should not happen).
pub(crate) fn array_buffer_byte_length(vm: &VmInner, id: ObjectId) -> usize {
    vm.body_data.get(&id).map_or(0, |b| b.len())
}

/// Return the full backing byte slice as an `Arc<[u8]>`, cheaply
/// cloning the reference-counted handle.  Used by the Body mixin
/// to ferry bytes back into `VmInner::body_data` when wrapping the
/// buffer in a new Response / Request body.
pub(crate) fn array_buffer_bytes(vm: &VmInner, id: ObjectId) -> Arc<[u8]> {
    vm.body_data
        .get(&id)
        .cloned()
        .unwrap_or_else(|| Arc::from(&[][..]))
}

/// Allocate an `ArrayBuffer` instance whose bytes are `bytes`.
/// Used by `.slice()` and by the Body mixin's `.arrayBuffer()`.
pub(crate) fn create_array_buffer_from_bytes(vm: &mut VmInner, bytes: Arc<[u8]>) -> ObjectId {
    let proto = vm.array_buffer_prototype;
    let id = vm.alloc_object(Object {
        kind: ObjectKind::ArrayBuffer,
        storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
        prototype: proto,
        extensible: true,
    });
    if !bytes.is_empty() {
        vm.body_data.insert(id, bytes);
    }
    id
}

/// ES §7.1.22 `ToIndex` narrowed to `usize` for the `ArrayBuffer`
/// `[[ArrayBufferByteLength]]` slot.  Routes the spec-wide
/// `[0, 2^53)` arithmetic through [`coerce::to_index_u64`] so the
/// V8-shaped error message and width stays in lockstep with the
/// other `ToIndex` callers, then layers a 32-bit-host safety check
/// on top: the constructor's backing-store allocation
/// (`vec![0_u8; length]`) needs the length to fit in `usize`, so
/// reject above-platform-max values up front with a spec-shaped
/// `RangeError` rather than letting the alloc itself abort the
/// process on capacity failure.  On 64-bit hosts that branch is
/// provably dead because the canonical helper already capped the
/// value at `2^53 − 1 < usize::MAX`.
fn to_index_for_array_buffer(
    ctx: &mut NativeContext<'_>,
    val: JsValue,
    what: &str,
) -> Result<usize, VmError> {
    let as_u64 = coerce::to_index_u64(ctx, val, "Failed to construct 'ArrayBuffer'", what)?;
    if as_u64 > usize::MAX as u64 {
        return Err(VmError::range_error(format!(
            "Failed to construct 'ArrayBuffer': {what} exceeds the maximum supported length on this platform"
        )));
    }
    #[allow(clippy::cast_possible_truncation)]
    Ok(as_u64 as usize)
}

// ---------------------------------------------------------------------------
// Constructor
// ---------------------------------------------------------------------------

/// `new ArrayBuffer(length)` (ES2020 §24.1.2).
fn native_array_buffer_constructor(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(
            "Failed to construct 'ArrayBuffer': Please use the 'new' operator",
        ));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };

    let length = match args.first().copied() {
        Some(JsValue::Undefined) | None => 0,
        Some(v) => to_index_for_array_buffer(ctx, v, "length")?,
    };

    // Promote the pre-allocated Ordinary instance to ArrayBuffer.
    // The `do_new`-allocated receiver already carries
    // `new.target.prototype`, so we must not touch `prototype` here
    // (PR5a2 R7.2/R7.3 lesson — subclass chain preservation).
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::ArrayBuffer;
    if length > 0 {
        // Allocate a zero-filled Arc<[u8]>.  Single allocation via
        // `vec![0u8; length].into()` avoids the intermediate Vec→Box
        // shuffle.
        let bytes: Arc<[u8]> = vec![0_u8; length].into();
        ctx.vm.body_data.insert(inst_id, bytes);
    }
    Ok(JsValue::Object(inst_id))
}

// ---------------------------------------------------------------------------
// Statics
// ---------------------------------------------------------------------------

/// `ArrayBuffer.isView(arg)` (ES §25.1.4.3).  Returns `true` for
/// TypedArray / DataView instances, `false` otherwise.  Never
/// throws; never coerces.
fn native_array_buffer_is_view(
    ctx: &mut NativeContext<'_>,
    _this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let arg = args.first().copied().unwrap_or(JsValue::Undefined);
    let result = matches!(
        arg,
        JsValue::Object(id) if matches!(
            ctx.vm.get_object(id).kind,
            ObjectKind::TypedArray { .. } | ObjectKind::DataView { .. }
        )
    );
    Ok(JsValue::Boolean(result))
}

// ---------------------------------------------------------------------------
// Accessors
// ---------------------------------------------------------------------------

fn native_array_buffer_get_byte_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_array_buffer_this(ctx, this, "byteLength")?;
    #[allow(clippy::cast_precision_loss)]
    let len = array_buffer_byte_length(ctx.vm, id) as f64;
    Ok(JsValue::Number(len))
}

// ---------------------------------------------------------------------------
// Methods
// ---------------------------------------------------------------------------

/// `ArrayBuffer.prototype.slice(begin?, end?)` (ES2020 §24.1.4.3).
///
/// Index resolution mirrors `Array.prototype.slice`:
/// - negative indices count from the end (`begin < 0` → `len + begin`).
/// - `undefined` end → `len`.
/// - out-of-range indices clamp to `[0, len]`.
fn native_array_buffer_slice(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_array_buffer_this(ctx, this, "slice")?;
    let len = array_buffer_byte_length(ctx.vm, id);
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

    let start = relative_index(begin, len_f);
    let stop = relative_index(end, len_f);
    let final_len = stop.saturating_sub(start);

    // Copy the slice into a fresh `Arc<[u8]>`.  Partial-share of an
    // `Arc<[u8]>` sub-range requires per-range allocation until the
    // backing store is refactored to support shared slices (planned
    // for the later TypedArray tranche of M4-12 — the current layer
    // is ArrayBuffer-only and byte-copy-on-slice is acceptable
    // because the measurable cost lands only when a script actually
    // slices large buffers).
    let bytes: Arc<[u8]> = if final_len == 0 {
        Arc::from(&[][..])
    } else {
        let src = ctx
            .vm
            .body_data
            .get(&id)
            .cloned()
            .unwrap_or_else(|| Arc::from(&[][..]));
        Arc::from(&src[start..stop])
    };
    let new_id = create_array_buffer_from_bytes(ctx.vm, bytes);
    Ok(JsValue::Object(new_id))
}

/// Clamp `n` to `[0, len]`, applying the spec
/// `ToIntegerOrInfinity` truncation first (ES §7.1.5 —
/// sign-preserving truncation toward zero, NaN → 0) so fractional
/// arguments match browser semantics: `slice(-1.9)` is `slice(-1)`
/// not `slice(-1.9)`.  Matches the slice-range helper used by
/// `ArrayBuffer.prototype.slice` (ES §25.1.5.3) and
/// `Blob.prototype.slice` (File API §3.2.3).  Shared here so
/// `blob.rs` doesn't re-implement the same function — both
/// callers live under `vm::host` (R24.1).  Thin usize-typed wrapper
/// around [`super::super::coerce::relative_index_f64`]; the clamp at
/// the canonical helper guarantees `0.0 <= clamped <= len`, so the
/// final `as usize` cast is exact when `len` originated from a
/// `usize` value.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(super) fn relative_index(n: f64, len: f64) -> usize {
    coerce::relative_index_f64(n, len) as usize
}
