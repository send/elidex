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
//! call followed by `.hyphenated().to_string()`.
//!
//! Phase 0b ships only the skeleton: prototype and brand-check
//! stubs, plus `window.crypto` data-property wiring.  Phase 1 / 2
//! land the `getRandomValues` / `randomUUID` natives; Phase 3
//! (digest) lives in [`super::subtle_crypto`].
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
    JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey, PropertyStorage,
    PropertyValue, VmError,
};
use super::super::VmInner;

impl VmInner {
    /// Allocate `Crypto.prototype` chained to `Object.prototype`,
    /// install the `getRandomValues` / `randomUUID` method natives +
    /// the `subtle` accessor, expose the `Crypto` constructor stub
    /// on `globalThis`, eagerly construct the per-VM `Crypto`
    /// wrapper, and install it as the `globalThis.crypto` data
    /// property.
    ///
    /// Called from `register_globals()` after `register_prototypes`
    /// (which populates `object_prototype`) AND after
    /// `register_subtle_crypto_global` (so the `subtle` accessor
    /// can reference `subtle_crypto_prototype` for its
    /// `alloc_or_cached_subtle_crypto` call).
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` or `subtle_crypto_prototype`
    /// is `None` — would mean the call-order invariant from
    /// `register_globals()` was violated.
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

        // Phase 0b: method + accessor natives land in Phase 1 / 2 / 3.
        // Skeleton only — the prototype exists for brand-check parity
        // and the `Crypto` global / `window.crypto` data property below.

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
        // Window getter; phase 3+ can promote to a `Window.prototype`
        // accessor without breaking the JS-visible shape (the
        // descriptor stays `{value, writable, configurable}` either
        // way for a data prop, and tests use `globalThis.crypto`
        // directly rather than `Object.getOwnPropertyDescriptor`).
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
/// by every `Crypto.prototype.*` method native; Phase 0b has no call
/// sites yet, suppressing dead-code lints until Phase 1 wires
/// `getRandomValues` / `randomUUID`.
#[allow(dead_code)]
pub(super) fn require_crypto_this(
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
