// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! `Crypto` interface (WebCrypto §10) — VM thin binding to the
//! `window.crypto` singleton.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this file contains only the
//! engine-bound responsibilities: prototype install, brand check,
//! and OS-CSPRNG / UUID generation.  There is no engine-independent
//! algorithm to delegate to — `getRandomValues` is a single
//! `getrandom::fill` call against the receiver's BufferSource
//! bytes, and `randomUUID` is a single `uuid::Uuid::new_v4`
//! call followed by `.hyphenated().to_string()`.  `digest` lives
//! in the sibling [`super::subtle_crypto`] module.
//!
//! ## Singleton storage
//!
//! - [`VmInner::crypto_instance`][]: cached `[SameObject]` `Crypto`
//!   wrapper installed as the `globalThis.crypto` data property at
//!   [`VmInner::register_crypto_global`] time.  Cleared on
//!   `Vm::unbind` so a retained reference is dropped after the next
//!   bind cycle.
//! - [`VmInner::crypto_prototype`][]: `Crypto.prototype` chained to
//!   `Object.prototype`; rooted via the proto-roots array in
//!   `vm/gc/collect.rs` so `delete globalThis.Crypto` cannot collect
//!   the prototype that retained instances still chain to.
//!
//! ## GC interaction
//!
//! [`ObjectKind::Crypto`] is payload-free.  The singleton is rooted
//! via `VmInner::crypto_instance` (mark-roots step in
//! `vm/gc/collect.rs`); trace fan-out is a no-op (see
//! `vm/gc/trace.rs`).

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{
    ElementKind, JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey,
    PropertyStorage, PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};

impl VmInner {
    /// Allocate `Crypto.prototype` chained to `Object.prototype`,
    /// install the `getRandomValues` / `randomUUID` method natives +
    /// the `subtle` accessor, expose the `Crypto` constructor stub
    /// on `globalThis`, eagerly construct the per-VM `Crypto`
    /// wrapper, and install it as the `globalThis.crypto` data
    /// property.
    ///
    /// Called from `register_globals()` after `register_prototypes`
    /// (which populates `object_prototype`).  The `subtle` accessor
    /// reads `subtle_crypto_prototype` lazily on the first JS
    /// invocation of `crypto.subtle`, so `register_subtle_crypto_global`
    /// only needs to run before that — not before this function.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — would mean the
    /// call-order invariant from `register_globals()` was violated.
    pub(in crate::vm) fn register_crypto_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_crypto_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        // `getRandomValues` + `randomUUID` methods (WebCrypto
        // §11.1 / §11.5).
        let methods: [(_, NativeFn); 2] = [
            (
                self.well_known.get_random_values,
                native_crypto_get_random_values as NativeFn,
            ),
            (self.well_known.random_uuid, native_crypto_random_uuid),
        ];
        for (name_sid, func) in methods {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }

        // `subtle` accessor (WebCrypto §10
        // `readonly attribute SubtleCrypto subtle` with [SameObject]).
        // Accessor (NOT data prop) so `Object.getOwnPropertyDescriptor(
        // Crypto.prototype, 'subtle')` yields `{get, enumerable,
        // configurable}` rather than `{value, writable}`.
        self.install_accessor_pair(
            proto_id,
            self.well_known.subtle,
            native_crypto_get_subtle,
            None,
            shape::PropertyAttrs::WEBIDL_RO_ACCESSOR,
        );

        self.crypto_prototype = Some(proto_id);

        // `Crypto` constructor stub — throws on call/construct, but
        // is required as a global so `crypto instanceof Crypto` and
        // `Crypto.prototype` parity work (WebIDL §10 + browser-
        // observed behaviour).
        let ctor = self.create_constructable_function("Crypto", native_crypto_illegal_ctor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            shape::PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            shape::PropertyAttrs::METHOD,
        );
        let name_sid = self.well_known.crypto_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));

        // `globalThis.crypto` — install eagerly as a data property.
        // Matches the `navigator` / `performance` precedent of
        // exposing the singleton on `globalThis` rather than via a
        // Window getter.  This is the chosen design (not a temporary
        // shim): there is no concrete trigger to migrate to a
        // `Window.prototype` accessor, and doing so would be a
        // JS-observable change to the descriptor shape (data prop
        // `{value, writable, configurable}` vs accessor
        // `{get, enumerable, configurable}`) that
        // `Object.getOwnPropertyDescriptor(globalThis, 'crypto')`
        // consumers would notice.
        let instance_id = self.alloc_or_cached_crypto();
        let crypto_key = PropertyKey::String(self.well_known.crypto_accessor);
        self.define_shaped_property(
            self.global_object,
            crypto_key,
            PropertyValue::Data(JsValue::Object(instance_id)),
            shape::PropertyAttrs::WEBIDL_RO,
        );
    }

    /// Return the per-VM `Crypto` `[SameObject]` wrapper, allocating
    /// it on the first call.  Eagerly invoked from
    /// `register_crypto_global` so `globalThis.crypto` is reachable
    /// from the start of script execution.  Subsequent calls return
    /// the cached `ObjectId`.
    ///
    /// Re-allocates after `Vm::unbind` (which clears
    /// `crypto_instance` for GC-root hygiene); a JS reference
    /// retained across rebind continues to brand-check successfully,
    /// and its methods will use the still-rooted prototype.
    pub(in crate::vm) fn alloc_or_cached_crypto(&mut self) -> ObjectId {
        if let Some(id) = self.crypto_instance {
            return id;
        }
        let proto = self
            .crypto_prototype
            .expect("alloc_or_cached_crypto before register_crypto_global");
        let id = self.alloc_object(Object {
            kind: ObjectKind::Crypto,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        });
        self.crypto_instance = Some(id);
        id
    }

    /// Clear the per-VM `Crypto` / `SubtleCrypto` singleton caches.
    /// Called from `Vm::unbind` for GC-root hygiene; the wrappers
    /// are payload-free so there is no data leak (unlike Storage's
    /// origin-keyed concern), but dropping the roots lets the
    /// wrappers be collected and re-allocated lazily after the
    /// next bind.
    pub(in crate::vm) fn clear_crypto_instance_cache(&mut self) {
        self.crypto_instance = None;
        self.subtle_crypto_instance = None;
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

/// Confirm `this` is a `Crypto` instance, returning a TypeError with
/// the spec-conformant "Illegal invocation" wording otherwise.  Used
/// by every `Crypto.prototype.*` method native.
fn require_crypto_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Crypto': Illegal invocation"
        )));
    };
    if !matches!(ctx.vm.get_object(id).kind, ObjectKind::Crypto) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'Crypto': Illegal invocation"
        )));
    }
    Ok(id)
}

// ---------------------------------------------------------------------------
// Constructor stub — `new Crypto()` throws per WebIDL §10
// ---------------------------------------------------------------------------

fn native_crypto_illegal_ctor(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Err(VmError::type_error(
        "Failed to construct 'Crypto': Illegal constructor",
    ))
}

// ---------------------------------------------------------------------------
// `Crypto.prototype.getRandomValues(view)` (WebCrypto §11.1)
// ---------------------------------------------------------------------------

/// Maximum byte length accepted by `getRandomValues` per WebCrypto
/// §11.1 step 1.  Views with `byteLength > QUOTA_EXCEEDED_LIMIT`
/// reject with `QuotaExceededError`; the boundary itself (==
/// `QUOTA_EXCEEDED_LIMIT`) is allowed.
const QUOTA_EXCEEDED_LIMIT: u32 = 65_536;

fn native_crypto_get_random_values(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_crypto_this(ctx, this, "getRandomValues")?;
    let view_val = args.first().copied().unwrap_or(JsValue::Undefined);

    // §11.1 step 1 — type check.  Allow-list of integer TypedArrays
    // (Int8/Uint8/Uint8Clamped/Int16/Uint16/Int32/Uint32/BigInt64/
    // BigUint64); Float32/Float64/DataView/non-TypedArray reject
    // with TypeError (NOT TypeMismatchError DOMException — modern
    // WebCrypto + WPT spec wording).  Matches Chrome / Firefox.
    let JsValue::Object(view_id) = view_val else {
        return Err(VmError::type_error(
            "Failed to execute 'getRandomValues' on 'Crypto': \
             parameter 1 is not of type '(Int8Array or Int16Array or \
             Int32Array or BigInt64Array or Uint8Array or Uint16Array \
             or Uint32Array or BigUint64Array or Uint8ClampedArray)'",
        ));
    };
    let (buffer_id, byte_offset, byte_length) = match ctx.vm.get_object(view_id).kind {
        ObjectKind::TypedArray {
            element_kind,
            buffer_id,
            byte_offset,
            byte_length,
        } => match element_kind {
            ElementKind::Int8
            | ElementKind::Uint8
            | ElementKind::Uint8Clamped
            | ElementKind::Int16
            | ElementKind::Uint16
            | ElementKind::Int32
            | ElementKind::Uint32
            | ElementKind::BigInt64
            | ElementKind::BigUint64 => (buffer_id, byte_offset, byte_length),
            ElementKind::Float32 | ElementKind::Float64 => {
                return Err(VmError::type_error(
                    "Failed to execute 'getRandomValues' on 'Crypto': \
                     The provided ArrayBufferView is of type 'Float' which is not an integer array type.",
                ));
            }
        },
        _ => {
            return Err(VmError::type_error(
                "Failed to execute 'getRandomValues' on 'Crypto': \
                 parameter 1 is not of type '(Int8Array or Int16Array or \
                 Int32Array or BigInt64Array or Uint8Array or Uint16Array \
                 or Uint32Array or BigUint64Array or Uint8ClampedArray)'",
            ));
        }
    };

    // WebIDL `BufferSource` boundary detach check per ECMA-262
    // §25.1.3.4 — `getRandomValues` accepts an `ArrayBufferView`
    // (typedef `[AllowShared] ArrayBufferView`) whose
    // `[[ViewedArrayBuffer]]` must not be detached.  Without this
    // check, the call would silently no-op on a zero-length
    // detached view, divergent from browser behaviour (Chrome /
    // Firefox both throw).  Placed before the §11.1 step 2 quota
    // check so a detached view does not surface a misleading
    // `QuotaExceededError` on the `byte_length == 0` path.
    if super::array_buffer::is_detached_buffer(ctx.vm, buffer_id) {
        return Err(VmError::type_error(
            "Failed to execute 'getRandomValues' on 'Crypto': \
             The provided ArrayBufferView is detached.",
        ));
    }

    // §11.1 step 2 — quota check.  Boundary value is allowed.
    if byte_length > QUOTA_EXCEEDED_LIMIT {
        return Err(VmError::dom_exception(
            ctx.vm.well_known.dom_exc_quota_exceeded_error,
            format!(
                "Failed to execute 'getRandomValues' on 'Crypto': \
                 The ArrayBufferView's byte length ({byte_length}) \
                 exceeds the number of bytes of entropy available via \
                 this API ({QUOTA_EXCEEDED_LIMIT})."
            ),
        ));
    }

    // §11.1 step 3 — zero-length short-circuit.  Skip the body_data
    // mutation entirely so the entry is NOT materialised by a no-op
    // write (mirrors `byte_io::write_at` empty-slice early-return —
    // `body_data.contains_key(&id)` is consulted by other call sites
    // as a "carries bytes?" signal).
    if byte_length == 0 {
        return Ok(view_val);
    }

    // §11.1 step 4 — fill view bytes with cryptographically strong
    // random.  Allocate into a stack-friendly temp `Vec<u8>` rather
    // than borrowing `body_data` mutably (which would conflict with
    // the `ctx.vm` borrow held by the brand check).  Up to 64 KiB =
    // well within heap budget for a one-shot call.
    let byte_len_usize = byte_length as usize;
    let abs = byte_offset as usize;
    let mut bytes = vec![0_u8; byte_len_usize];
    getrandom::fill(&mut bytes).map_err(|e| {
        VmError::type_error(format!(
            "Failed to execute 'getRandomValues' on 'Crypto': \
             OS CSPRNG failure ({e})"
        ))
    })?;
    super::byte_io::write_at(&mut ctx.vm.body_data, buffer_id, abs, &bytes);

    // §11.1 step 5 — return the SAME view receiver (identity per
    // IDL `ArrayBufferView getRandomValues(ArrayBufferView array)`).
    Ok(view_val)
}

// ---------------------------------------------------------------------------
// `Crypto.prototype.randomUUID()` (WebCrypto §11.5)
// ---------------------------------------------------------------------------

fn native_crypto_random_uuid(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_crypto_this(ctx, this, "randomUUID")?;
    // §11.5: return a v4 UUID per RFC 4122 §4.4, lowercase, hyphenated.
    // `uuid::Uuid::new_v4()` consumes the OS CSPRNG via `getrandom`
    // (configured via the v4 feature) and `.hyphenated()` emits the
    // canonical 8-4-4-4-12 form with the version (`4`) and variant
    // (`8`/`9`/`a`/`b`) bits set at the spec-mandated positions.
    let uuid = uuid::Uuid::new_v4();
    let s = uuid.hyphenated().to_string();
    let sid = ctx.vm.strings.intern(&s);
    Ok(JsValue::String(sid))
}

// ---------------------------------------------------------------------------
// `Crypto.prototype.subtle` accessor (WebCrypto §10, [SameObject])
// ---------------------------------------------------------------------------

fn native_crypto_get_subtle(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    require_crypto_this(ctx, this, "subtle")?;
    let id = ctx.vm.alloc_or_cached_subtle_crypto();
    Ok(JsValue::Object(id))
}
