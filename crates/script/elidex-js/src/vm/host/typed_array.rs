//! `%TypedArray%` + concrete subclasses (ES2024 §23.2).
//!
//! `%TypedArray%` is an abstract base class — not exposed as a global,
//! reachable only via `Object.getPrototypeOf(Uint8Array)` etc. — that
//! carries shared IDL attrs (`buffer` / `byteOffset` / `byteLength` /
//! `length`) plus the prototype method suite (`fill` / `set` /
//! `subarray` / …).  11 concrete subclasses (`Int8Array` / `Uint8Array`
//! / `Uint8ClampedArray` / `Int16Array` / `Uint16Array` / `Int32Array`
//! / `Uint32Array` / `Float32Array` / `Float64Array` / `BigInt64Array`
//! / `BigUint64Array`) chain their prototype to `%TypedArray%.prototype`
//! and differ only by the [`super::super::value::ElementKind`] tag
//! baked into each instance's [`ObjectKind::TypedArray`] variant.
//!
//! ```text
//! new Uint8Array(n)           ObjectKind::TypedArray { element_kind: Uint8, … }
//!   → Uint8Array.prototype
//!     → %TypedArray%.prototype
//!       → Object.prototype
//! ```
//!
//! ## Byte-order convention
//!
//! TypedArray indexed reads / writes use **little-endian byte order
//! unconditionally** — an elidex implementation choice for
//! cross-platform determinism.  `IsLittleEndian()` (ES §25.1.3.1) is
//! implementation-defined, so a constant choice is spec-compliant.
//! [`super::data_view::DataView`] exposes both endiannesses explicitly
//! via its `littleEndian` argument (ES §25.3.4, default `false`).
//!
//! ## Backing storage
//!
//! A TypedArray is a **view**: the bytes live in the underlying
//! [`ObjectKind::ArrayBuffer`] (shared [`super::super::VmInner::body_data`]
//! entry), and every view over the same buffer mutates the same
//! bytes.  The view's `[[ByteOffset]]` / `[[ByteLength]]` slots
//! stored inline on `ObjectKind::TypedArray` translate JS indices to
//! buffer offsets.  No side-table is needed because all four spec
//! slots are immutable after construction in this PR — `transfer()`
//! / `resize()` / `detached` tracking (ES2024) are deferred to the
//! M4-12 cutover-residual tranche.
//!
//! ## Scope
//!
//! This module currently implements the C1 scaffolding only:
//! - [`ObjectKind::TypedArray`] / [`ObjectKind::DataView`] variants in
//!   [`super::super::value`].
//! - [`super::super::value::ElementKind`] enum + helpers.
//! - `%TypedArray%.prototype` allocation (this module, this file).
//!
//! Subclass ctors + per-subclass `.of` / `.from` statics + indexed
//! element access + prototype methods land in PR5-typed-array §C2-C4.
//! `%TypedArray%` abstract ctor + `@@toStringTag` + `@@species` land
//! alongside the first subclass in §C2 (they need a subclass
//! instance to be JS-observable).

#![cfg(feature = "engine")]

use super::super::shape;
use super::super::value::{Object, ObjectKind, PropertyStorage};
use super::super::VmInner;

impl VmInner {
    /// Allocate `%TypedArray%.prototype` chained to `Object.prototype`.
    /// Must run during `register_globals()` after
    /// `register_prototypes` populates `object_prototype` — and
    /// before any subclass registration so the subclass-prototype
    /// chain can splice `%TypedArray%.prototype` in.
    ///
    /// The prototype is an empty [`ObjectKind::Ordinary`] in this
    /// commit.  The abstract `%TypedArray%` constructor (which throws
    /// in both call- and new-mode per ES §23.2.1.1), the
    /// `@@toStringTag` / `@@species` getters, and the four generic
    /// accessors (`buffer` / `byteOffset` / `byteLength` / `length`)
    /// install alongside the first concrete subclass in
    /// PR5-typed-array §C2 — at which point they become JS-observable
    /// and testable through a `Uint8Array` instance.
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — indicates a
    /// mis-ordered registration pass.
    pub(in crate::vm) fn register_typed_array_prototype_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_typed_array_prototype_global called before register_prototypes");

        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.typed_array_prototype = Some(proto_id);
    }
}
