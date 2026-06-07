// boa skip: VM-only surface; the legacy boa runtime
// (`crates/script/elidex-js-boa/`) is on the deletion path per
// `m4-12-platform-gap-roadmap.md` §E-2 Round 20 PR7.  See
// `memory/project_boa_runtime_deletion.md`.

//! `SubtleCrypto` interface (WebCrypto §14) — VM thin binding to
//! the lazy-allocated `crypto.subtle` singleton.
//!
//! ## Layering
//!
//! Per CLAUDE.md "Layering mandate", this module contains only the
//! engine-bound responsibilities: prototype install, brand check, and
//! marshalling JS values ↔ the engine-independent `elidex-api-crypto`
//! API (algorithm normalization, key validation, HMAC, digest, and JWK
//! all live in the crate).  BufferSource coercion is reused via
//! [`super::text_encoding::extract_buffer_source_bytes`].
//!
//! Current scope (`#11-crypto-subtle-full` PR-1 + PR-2 + PR-3a + PR-3b):
//! `digest`, the `CryptoKey` lifecycle, the HMAC vertical (`generateKey` /
//! `importKey` / `exportKey` / `sign` / `verify`), the AES-GCM /
//! AES-CBC / AES-CTR vertical (`generateKey` / `importKey` /
//! `exportKey` / `encrypt` / `decrypt`), the HKDF / PBKDF2 derive
//! vertical (`importKey` / `deriveBits` / `deriveKey`), and the wrap
//! vertical (`wrapKey` / `unwrapKey`, AES-KW + the AES-GCM/CBC/CTR
//! encrypt/decrypt fallback).  ECDSA/ECDH (PR-4) and RSA (PR-5) extend
//! the crate registry by adding rows.
//!
//! ## Submodules
//!
//! The combined surface exceeds the 1000-line file convention, so it
//! is split into a directory module:
//!
//! - this `mod.rs` — singleton registration / `[SameObject]` wrapper
//!   allocation, the receiver brand check, and the shared [`run_op`]
//!   Promise harness (`run_op` + `settle_promise`).
//! - [`marshal`] — JS-value → `elidex-api-crypto` input marshalling
//!   (algorithm-identifier conversion + normalization inputs, key-usage
//!   / format / JWK conversion, the `[EnforceRange]` length coercion,
//!   and the `oct`-JWK builder).
//! - [`ops`] — the twelve operation natives (`digest` + the HMAC
//!   `generateKey` / `importKey` / `exportKey` / `sign` / `verify`
//!   vertical + the AES `encrypt` / `decrypt` + the KDF `deriveBits` /
//!   `deriveKey` + the `wrapKey` / `unwrapKey` wrap vertical) plus the
//!   `AlgorithmError` → DOMException mapping.
//!
//! ## Singleton storage
//!
//! - [`VmInner::subtle_crypto_instance`][]: cached `[SameObject]`
//!   `SubtleCrypto` wrapper returned by the `Crypto.prototype.subtle`
//!   accessor.  Lazily allocated via
//!   [`VmInner::alloc_or_cached_subtle_crypto`] on the first
//!   `crypto.subtle` read.  Cleared on `Vm::unbind` for GC-root
//!   hygiene (the wrapper is stateless, so re-allocation is cheap).
//! - [`VmInner::subtle_crypto_prototype`][]: `SubtleCrypto.prototype`
//!   chained to `Object.prototype`; rooted via the proto-roots
//!   array in `vm/gc/collect.rs`.
//!
//! ## GC interaction
//!
//! [`ObjectKind::SubtleCrypto`] is payload-free.  Singleton rooted
//! via `VmInner::subtle_crypto_instance` (mark-roots step in
//! `vm/gc/collect.rs`); trace fan-out is a no-op (see
//! `vm/gc/trace.rs`).

#![cfg(feature = "engine")]

mod marshal;
mod ops;

use super::super::natives_promise::create_promise;
use super::super::shape;
use super::super::value::{
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::{NativeFn, VmInner};
use super::blob::{reject_promise_sync, resolve_promise_sync};
use ops::{
    native_subtle_crypto_decrypt, native_subtle_crypto_derive_bits,
    native_subtle_crypto_derive_key, native_subtle_crypto_digest, native_subtle_crypto_encrypt,
    native_subtle_crypto_export_key, native_subtle_crypto_generate_key,
    native_subtle_crypto_import_key, native_subtle_crypto_sign, native_subtle_crypto_unwrap_key,
    native_subtle_crypto_verify, native_subtle_crypto_wrap_key,
};

impl VmInner {
    /// Allocate `SubtleCrypto.prototype` chained to `Object.prototype`,
    /// install the `digest` method native, and expose the
    /// `SubtleCrypto` constructor stub on `globalThis`.
    ///
    /// Called from `register_globals()` after `register_prototypes`.
    /// Ordering relative to `register_crypto_global` does NOT matter
    /// for the prototype install itself — `Crypto.prototype.subtle`'s
    /// getter reads `subtle_crypto_prototype` lazily on the first
    /// JS invocation of `crypto.subtle`, which always happens after
    /// `register_globals()` has finished both registrations.  The
    /// current ordering (subtle before crypto, see
    /// `vm/globals.rs::register_globals`) is alphabetical / topological-
    /// hint convenience, not a correctness requirement.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — call-order
    /// invariant from `register_globals()` violated.
    pub(in crate::vm) fn register_subtle_crypto_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_subtle_crypto_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });

        // `SubtleCrypto.prototype` operation natives (WebCrypto §14.3).
        let methods: [(_, NativeFn); 12] = [
            (
                self.well_known.digest,
                native_subtle_crypto_digest as NativeFn,
            ),
            (
                self.well_known.generate_key,
                native_subtle_crypto_generate_key,
            ),
            (self.well_known.import_key, native_subtle_crypto_import_key),
            (self.well_known.export_key, native_subtle_crypto_export_key),
            (self.well_known.sign, native_subtle_crypto_sign),
            (self.well_known.verify, native_subtle_crypto_verify),
            (self.well_known.encrypt, native_subtle_crypto_encrypt),
            (self.well_known.decrypt, native_subtle_crypto_decrypt),
            (
                self.well_known.derive_bits,
                native_subtle_crypto_derive_bits,
            ),
            (self.well_known.derive_key, native_subtle_crypto_derive_key),
            (self.well_known.wrap_key, native_subtle_crypto_wrap_key),
            (self.well_known.unwrap_key, native_subtle_crypto_unwrap_key),
        ];
        for (name_sid, func) in methods {
            self.install_native_method(proto_id, name_sid, func, shape::PropertyAttrs::METHOD);
        }

        self.subtle_crypto_prototype = Some(proto_id);

        // `SubtleCrypto` declares no constructor operation — registered as
        // IllegalConstructor so both call/construct throw at the gate
        // (Web Cryptography API §14 SubtleCrypto interface).
        let ctor = self.create_illegal_constructor_function(
            "SubtleCrypto",
            super::super::value::native_illegal_constructor_unreachable,
        );
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
        let name_sid = self.well_known.subtle_crypto_global;
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }

    /// Return the per-VM `SubtleCrypto` `[SameObject]` wrapper,
    /// allocating it on the first `Crypto.prototype.subtle` accessor
    /// read.  Mirrors the [`super::dom_selection_proto`]
    /// `alloc_or_cached_selection` shape: cached singleton, payload-
    /// free, prototype-via-`subtle_crypto_prototype`.
    ///
    /// Re-allocates after `Vm::unbind` (which clears
    /// `subtle_crypto_instance` for GC-root hygiene); the prototype
    /// stays live so retained JS references continue to brand-check
    /// and dispatch through the same `digest` native.
    pub(in crate::vm) fn alloc_or_cached_subtle_crypto(&mut self) -> ObjectId {
        if let Some(id) = self.subtle_crypto_instance {
            return id;
        }
        let proto = self
            .subtle_crypto_prototype
            .expect("alloc_or_cached_subtle_crypto before register_subtle_crypto_global");
        let id = self.alloc_object(Object {
            kind: ObjectKind::SubtleCrypto,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(proto),
            extensible: false,
        });
        self.subtle_crypto_instance = Some(id);
        id
    }
}

// ---------------------------------------------------------------------------
// Brand check
// ---------------------------------------------------------------------------

/// Confirm `this` is a `SubtleCrypto` instance, returning a
/// TypeError with the spec-conformant "Illegal invocation" wording
/// otherwise.  Used by every `SubtleCrypto.prototype.*` method
/// native (via [`run_op`]).
fn require_subtle_crypto_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &'static str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'SubtleCrypto': Illegal invocation"
        )));
    };
    if !matches!(ctx.vm.get_object(id).kind, ObjectKind::SubtleCrypto) {
        return Err(VmError::type_error(format!(
            "Failed to execute '{method}' on 'SubtleCrypto': Illegal invocation"
        )));
    }
    Ok(id)
}

// ---------------------------------------------------------------------------
// Promise harness shared by the operation natives
// ---------------------------------------------------------------------------

/// Run an operation body against a pre-rooted Promise, settling it.
/// Shared shape for the twelve operation natives (digest + the five HMAC ops +
/// AES encrypt / decrypt + KDF deriveBits / deriveKey + wrapKey / unwrapKey).
///
/// WebCrypto §14.3 reports **all** errors asynchronously, including the
/// Web IDL receiver brand check: a non-`SubtleCrypto` `this` (e.g.
/// `crypto.subtle.sign.call({}, …)`) must reject the returned Promise, not
/// throw synchronously.  So the brand check runs *inside* the settled
/// closure, after the Promise is created.
pub(super) fn run_op(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    method: &'static str,
    body: impl FnOnce(&mut NativeContext<'_>) -> Result<JsValue, VmError>,
) -> Result<JsValue, VmError> {
    let promise = create_promise(ctx.vm);
    let mut guard = ctx.vm.push_temp_root(JsValue::Object(promise));
    let mut rooted = NativeContext::new_call(&mut guard);
    let ctx = &mut rooted;
    let result = match require_subtle_crypto_this(ctx, this, method) {
        Ok(_) => body(ctx),
        Err(e) => Err(e),
    };
    settle_promise(ctx, promise, result);
    Ok(JsValue::Object(promise))
}

/// Settle a pre-allocated Promise with the result of an operation body.
/// Brand-check failures are handled before this; every other failure
/// (marshalling `VmError` or crate `AlgorithmError` already mapped to
/// `VmError`) rejects the Promise rather than throwing synchronously.
fn settle_promise(
    ctx: &mut NativeContext<'_>,
    promise: ObjectId,
    result: Result<JsValue, VmError>,
) {
    match result {
        Ok(value) => resolve_promise_sync(ctx.vm, promise, value),
        Err(e) => {
            let reason = ctx.vm.vm_error_to_thrown(&e);
            reject_promise_sync(ctx.vm, promise, reason);
        }
    }
}
