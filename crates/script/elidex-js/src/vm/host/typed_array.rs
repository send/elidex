//! `%TypedArray%` + concrete subclasses (ES2024 §23.2).
//!
//! `%TypedArray%` is an abstract base class — not exposed as a global,
//! reachable only via `Object.getPrototypeOf(Uint8Array)` etc. — that
//! carries shared IDL attrs (`buffer` / `byteOffset` / `byteLength` /
//! `length`) plus the prototype method suite (`fill` / `set` /
//! `subarray` / …, landing in PR5-typed-array §C4).  11 concrete
//! subclasses (`Int8Array` / `Uint8Array` / `Uint8ClampedArray` /
//! `Int16Array` / `Uint16Array` / `Int32Array` / `Uint32Array` /
//! `Float32Array` / `Float64Array` / `BigInt64Array` /
//! `BigUint64Array`) chain their prototype to `%TypedArray%.prototype`
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
//! [`super::data_view::DataView`] (PR5-typed-array §C5) exposes both
//! endiannesses explicitly via its `littleEndian` argument (ES
//! §25.3.4, default `false`).
//!
//! ## Backing storage
//!
//! A TypedArray is a **view**: the bytes live in the underlying
//! [`ObjectKind::ArrayBuffer`] (shared [`super::super::VmInner::body_data`]
//! entry), and every view over the same buffer mutates the same
//! bytes.  The view's `[[ByteOffset]]` / `[[ByteLength]]` slots
//! stored inline on `ObjectKind::TypedArray` translate JS indices to
//! buffer offsets.  No side-table is needed because all four spec
//! slots are immutable after construction in this PR —
//! `transfer()` / `resize()` / `detached` tracking (ES2024) are
//! deferred to the M4-12 cutover-residual tranche.

#![cfg(feature = "engine")]

use std::sync::Arc;

use super::super::coerce;
use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    ElementKind, JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey,
    PropertyStorage, PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};
use super::typed_array_ctor::{init_from_array_buffer, init_from_iterable, init_from_typed_array};

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

impl VmInner {
    /// Allocate `%TypedArray%.prototype`, the abstract
    /// `%TypedArray%` constructor, `@@toStringTag` / `@@species` /
    /// `constructor` links, and all 11 concrete subclass prototypes
    /// + constructors.  Must run during `register_globals()` after
    /// `register_array_buffer_global` (ArrayBuffer backs the
    /// TypedArray's bytes) and before
    /// `register_structured_clone_global` (so C6 clone arms can
    /// reach the subclass prototype ids).
    ///
    /// # Panics
    ///
    /// Panics if `object_prototype` is `None` — indicates a
    /// mis-ordered registration pass.
    pub(in crate::vm) fn register_typed_array_prototype_global(&mut self) {
        let object_proto = self
            .object_prototype
            .expect("register_typed_array_prototype_global called before register_prototypes");

        // Abstract `%TypedArray%.prototype` (ES §23.2.3).
        let proto_id = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(object_proto),
            extensible: true,
        });
        self.typed_array_prototype = Some(proto_id);
        self.install_typed_array_prototype_members(proto_id);

        // Abstract `%TypedArray%` constructor (ES §23.2.1).
        // Callable, constructable — but both paths unconditionally
        // throw TypeError per §23.2.1.1 ("Abstract class TypedArray
        // not directly constructable").  The function still exists
        // so `Object.getPrototypeOf(Uint8Array) === %TypedArray%`
        // holds and so `@@species` can live on it.
        let abstract_ctor =
            self.create_constructable_function("TypedArray", native_abstract_typed_array_ctor);
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            abstract_ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(proto_id)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            proto_id,
            ctor_key,
            PropertyValue::Data(JsValue::Object(abstract_ctor)),
            PropertyAttrs::METHOD,
        );

        // `%TypedArray%[@@species]` accessor — returns `this`
        // (ES §23.2.2.4).  Enables spec-correct species lookup for
        // allocating methods (`map` / `filter` / `slice` — installed
        // in C4), though the current minimal method set uses
        // identity constructor (same subclass as receiver).
        let species_getter =
            self.create_native_function("get [Symbol.species]", native_typed_array_species_get);
        self.define_shaped_property(
            abstract_ctor,
            PropertyKey::Symbol(self.well_known_symbols.species),
            PropertyValue::Accessor {
                getter: Some(species_getter),
                setter: None,
            },
            PropertyAttrs::ES_BUILTIN_ACCESSOR,
        );

        // Register each of the 11 concrete subclass prototypes +
        // constructors, chaining the prototype to the abstract's
        // prototype and the ctor to `%TypedArray%`.
        // Iteration order matches `ElementKind` declaration for
        // predictable global-install ordering.
        for entry in &SUBCLASS_TABLE {
            self.register_typed_array_subclass(entry, proto_id, abstract_ctor);
        }
    }

    /// Install the generic accessors (`buffer` / `byteOffset` /
    /// `byteLength` / `length`) and the `@@toStringTag` getter on
    /// `%TypedArray%.prototype`.  The four accessors read the
    /// authoritative spec slots carried inline on
    /// [`ObjectKind::TypedArray`], so they work uniformly across
    /// every subclass instance without per-subclass install.
    fn install_typed_array_prototype_members(&mut self, proto_id: ObjectId) {
        let accessors: [(StringId, NativeFn); 4] = [
            (
                self.well_known.buffer,
                native_typed_array_get_buffer as NativeFn,
            ),
            (
                self.well_known.byte_offset,
                native_typed_array_get_byte_offset as NativeFn,
            ),
            (
                self.well_known.byte_length,
                native_typed_array_get_byte_length as NativeFn,
            ),
            (
                self.well_known.length,
                native_typed_array_get_length as NativeFn,
            ),
        ];
        for (name_sid, getter_fn) in accessors {
            self.install_accessor_pair(
                proto_id,
                name_sid,
                getter_fn,
                None,
                PropertyAttrs::ES_BUILTIN_ACCESSOR,
            );
        }

        // `%TypedArray%.prototype[@@toStringTag]` getter (ES
        // §23.2.3.32): returns the subclass name string from
        // `[[TypedArrayName]]` (derived from `element_kind`), or
        // `undefined` if `this` lacks the brand (does NOT throw,
        // per spec — the getter silently yields undefined so
        // `Object.prototype.toString.call(foreign)` returns
        // `"[object Object]"` rather than error).
        let tag_getter =
            self.create_native_function("get [Symbol.toStringTag]", native_typed_array_to_tag_get);
        self.define_shaped_property(
            proto_id,
            PropertyKey::Symbol(self.well_known_symbols.to_string_tag),
            PropertyValue::Accessor {
                getter: Some(tag_getter),
                setter: None,
            },
            PropertyAttrs::ES_BUILTIN_ACCESSOR,
        );

        // Core method suite.  See `typed_array_methods` for the
        // spec-aligned bodies; each method is installed on
        // `%TypedArray%.prototype` (shared across all 11 subclasses
        // via prototype chain).  Names are pre-interned in
        // `WellKnownStrings` so this table doesn't pay the per-call
        // `strings.intern(...)` round-trip during `Vm::new`.
        let methods: [(StringId, NativeFn); 19] = [
            (
                self.well_known.fill,
                super::typed_array_methods::native_typed_array_fill as NativeFn,
            ),
            (
                self.well_known.subarray,
                super::typed_array_methods::native_typed_array_subarray as NativeFn,
            ),
            (
                self.well_known.slice,
                super::typed_array_methods::native_typed_array_slice as NativeFn,
            ),
            (
                self.well_known.values,
                super::typed_array_methods::native_typed_array_values as NativeFn,
            ),
            (
                self.well_known.keys,
                super::typed_array_methods::native_typed_array_keys as NativeFn,
            ),
            (
                self.well_known.entries,
                super::typed_array_methods::native_typed_array_entries as NativeFn,
            ),
            (
                self.well_known.set,
                super::typed_array_methods::native_typed_array_set as NativeFn,
            ),
            (
                self.well_known.copy_within,
                super::typed_array_methods::native_typed_array_copy_within as NativeFn,
            ),
            (
                self.well_known.reverse,
                super::typed_array_methods::native_typed_array_reverse as NativeFn,
            ),
            (
                self.well_known.index_of,
                super::typed_array_methods::native_typed_array_index_of as NativeFn,
            ),
            (
                self.well_known.last_index_of,
                super::typed_array_methods::native_typed_array_last_index_of as NativeFn,
            ),
            (
                self.well_known.includes,
                super::typed_array_methods::native_typed_array_includes as NativeFn,
            ),
            (
                self.well_known.at,
                super::typed_array_methods::native_typed_array_at as NativeFn,
            ),
            (
                self.well_known.join,
                super::typed_array_methods::native_typed_array_join as NativeFn,
            ),
            (
                self.well_known.for_each,
                super::typed_array_methods::native_typed_array_for_each as NativeFn,
            ),
            (
                self.well_known.every,
                super::typed_array_methods::native_typed_array_every as NativeFn,
            ),
            (
                self.well_known.some,
                super::typed_array_methods::native_typed_array_some as NativeFn,
            ),
            (
                self.well_known.find,
                super::typed_array_methods::native_typed_array_find as NativeFn,
            ),
            (
                self.well_known.find_index,
                super::typed_array_methods::native_typed_array_find_index as NativeFn,
            ),
        ];
        for (name_sid, fn_ptr) in methods {
            self.install_native_method(proto_id, name_sid, fn_ptr, PropertyAttrs::METHOD);
        }

        // `%TypedArray%.prototype[Symbol.iterator]` — spec-mandated
        // identity-equal to `.values` (ES §23.2.3.33).  Install
        // after `values` so we can reuse the same function id.
        let values_slot = super::super::coerce::get_property(
            self,
            proto_id,
            PropertyKey::String(self.well_known.values),
        );
        if let Some(super::super::coerce::PropertyResult::Data(JsValue::Object(values_fn))) =
            values_slot
        {
            self.define_shaped_property(
                proto_id,
                PropertyKey::Symbol(self.well_known_symbols.iterator),
                PropertyValue::Data(JsValue::Object(values_fn)),
                PropertyAttrs::METHOD,
            );
        }

        // `%TypedArray%.prototype.toString` — identity-equal to
        // `Array.prototype.toString` (ES §23.2.3.31 "same built-in
        // function object").  Install by copying the existing
        // function id rather than creating a new native, so
        // `Uint8Array.prototype.toString === Array.prototype.toString`.
        if let Some(array_proto) = self.array_prototype {
            let to_string_sid = self.strings.intern("toString");
            if let Some(super::super::coerce::PropertyResult::Data(JsValue::Object(
                array_to_string,
            ))) = super::super::coerce::get_property(
                self,
                array_proto,
                PropertyKey::String(to_string_sid),
            ) {
                self.define_shaped_property(
                    proto_id,
                    PropertyKey::String(to_string_sid),
                    PropertyValue::Data(JsValue::Object(array_to_string)),
                    PropertyAttrs::METHOD,
                );
            }
        }
    }

    /// Allocate one subclass `Xxx.prototype → %TypedArray%.prototype`
    /// + constructor `Xxx → %TypedArray%` (via prototype link),
    /// install `BYTES_PER_ELEMENT` on both ctor and prototype, and
    /// expose the ctor on `globals`.
    ///
    /// Per PR5b D4 per-interface brand wrappers lesson, each
    /// subclass gets its own static native fn so error messages
    /// carry the subclass name (e.g. `"Failed to construct
    /// 'Uint8Array': …"`).  The wrappers fan into a shared
    /// [`construct_typed_array`] impl parameterised by
    /// [`ElementKind`].
    fn register_typed_array_subclass(
        &mut self,
        entry: &SubclassEntry,
        abstract_proto: ObjectId,
        abstract_ctor: ObjectId,
    ) {
        let sub_proto = self.alloc_object(Object {
            kind: ObjectKind::Ordinary,
            storage: PropertyStorage::shaped(shape::ROOT_SHAPE),
            prototype: Some(abstract_proto),
            extensible: true,
        });
        self.subclass_array_prototypes[entry.element_kind.index()] = Some(sub_proto);

        let ctor = self.create_constructable_function(entry.name, entry.ctor_fn);
        // Prototype chain between subclass ctor and abstract ctor —
        // `Object.getPrototypeOf(Uint8Array) === %TypedArray%`
        // (ES §23.2.6).  `create_constructable_function` sets
        // `function_prototype` by default; override to the abstract
        // ctor.
        self.get_object_mut(ctor).prototype = Some(abstract_ctor);

        // Subclass prototype ↔ ctor cross-links.
        let proto_key = PropertyKey::String(self.well_known.prototype);
        self.define_shaped_property(
            ctor,
            proto_key,
            PropertyValue::Data(JsValue::Object(sub_proto)),
            PropertyAttrs::BUILTIN,
        );
        let ctor_key = PropertyKey::String(self.well_known.constructor);
        self.define_shaped_property(
            sub_proto,
            ctor_key,
            PropertyValue::Data(JsValue::Object(ctor)),
            PropertyAttrs::METHOD,
        );

        // `BYTES_PER_ELEMENT` on both prototype and constructor
        // (ES §23.2.6.1 / §23.2.7.1): `{writable: false,
        // enumerable: false, configurable: false}`.
        let bpe_attrs = PropertyAttrs {
            writable: false,
            enumerable: false,
            configurable: false,
            is_accessor: false,
        };
        let bpe_key = PropertyKey::String(self.well_known.bytes_per_element);
        let bpe_value = PropertyValue::Data(JsValue::Number(f64::from(
            entry.element_kind.bytes_per_element(),
        )));
        self.define_shaped_property(ctor, bpe_key, bpe_value, bpe_attrs);
        self.define_shaped_property(sub_proto, bpe_key, bpe_value, bpe_attrs);

        // Global ctor exposure (`Uint8Array`, `Int8Array`, …).
        let name_sid = (entry.global_name)(&self.well_known);
        self.globals.insert(name_sid, JsValue::Object(ctor));
    }
}

// ---------------------------------------------------------------------------
// Subclass dispatch table
// ---------------------------------------------------------------------------

/// Per-subclass registration data, keeping the 11 thin wrapper
/// functions (`native_uint8_array_ctor` et al.) in a compact table
/// rather than a match arm soup in `register_typed_array_prototype_global`.
/// Every field is `Copy` so the static array can live in `.rodata`.
/// The per-subclass prototype slot is addressed via
/// `element_kind.index()` against `VmInner::subclass_array_prototypes`,
/// so the table no longer carries a per-entry write closure.
#[derive(Clone, Copy)]
struct SubclassEntry {
    name: &'static str,
    element_kind: ElementKind,
    ctor_fn: NativeFn,
    global_name: fn(&super::super::well_known::WellKnownStrings) -> StringId,
}

/// Sized to `ElementKind::COUNT` so a missing or extra entry fails
/// to compile via the array length.  The const-time check below the
/// literal then enforces that each entry's `element_kind` matches
/// its position via `ElementKind::index()`, which makes duplicates
/// or out-of-order entries also refuse to compile.
static SUBCLASS_TABLE: [SubclassEntry; ElementKind::COUNT] = [
    SubclassEntry {
        name: "Int8Array",
        element_kind: ElementKind::Int8,
        ctor_fn: native_int8_array_ctor,
        global_name: |w| w.int8_array_global,
    },
    SubclassEntry {
        name: "Uint8Array",
        element_kind: ElementKind::Uint8,
        ctor_fn: native_uint8_array_ctor,
        global_name: |w| w.uint8_array_global,
    },
    SubclassEntry {
        name: "Uint8ClampedArray",
        element_kind: ElementKind::Uint8Clamped,
        ctor_fn: native_uint8_clamped_array_ctor,
        global_name: |w| w.uint8_clamped_array_global,
    },
    SubclassEntry {
        name: "Int16Array",
        element_kind: ElementKind::Int16,
        ctor_fn: native_int16_array_ctor,
        global_name: |w| w.int16_array_global,
    },
    SubclassEntry {
        name: "Uint16Array",
        element_kind: ElementKind::Uint16,
        ctor_fn: native_uint16_array_ctor,
        global_name: |w| w.uint16_array_global,
    },
    SubclassEntry {
        name: "Int32Array",
        element_kind: ElementKind::Int32,
        ctor_fn: native_int32_array_ctor,
        global_name: |w| w.int32_array_global,
    },
    SubclassEntry {
        name: "Uint32Array",
        element_kind: ElementKind::Uint32,
        ctor_fn: native_uint32_array_ctor,
        global_name: |w| w.uint32_array_global,
    },
    SubclassEntry {
        name: "Float32Array",
        element_kind: ElementKind::Float32,
        ctor_fn: native_float32_array_ctor,
        global_name: |w| w.float32_array_global,
    },
    SubclassEntry {
        name: "Float64Array",
        element_kind: ElementKind::Float64,
        ctor_fn: native_float64_array_ctor,
        global_name: |w| w.float64_array_global,
    },
    SubclassEntry {
        name: "BigInt64Array",
        element_kind: ElementKind::BigInt64,
        ctor_fn: native_bigint64_array_ctor,
        global_name: |w| w.bigint64_array_global,
    },
    SubclassEntry {
        name: "BigUint64Array",
        element_kind: ElementKind::BigUint64,
        ctor_fn: native_biguint64_array_ctor,
        global_name: |w| w.biguint64_array_global,
    },
];

// Compile-time check that every entry's `element_kind` matches its
// position via `ElementKind::index()`.  Together with the table's
// `[_; ElementKind::COUNT]` length, this rejects a missing entry, a
// duplicate variant, or an out-of-order entry — anything that would
// otherwise leave the install loop's `subclass_array_prototypes[index]`
// write pointing at the wrong slot.
const _: () = {
    let mut i = 0;
    while i < ElementKind::COUNT {
        assert!(
            SUBCLASS_TABLE[i].element_kind.index() == i,
            "SUBCLASS_TABLE entry index does not match ElementKind::index() — \
             check that entries appear in ElementKind variant order without duplicates"
        );
        i += 1;
    }
};

// ---------------------------------------------------------------------------
// Per-subclass constructor thin wrappers
// ---------------------------------------------------------------------------

// Per-subclass wrappers (vs a single generic ctor with a lookup) are
// what surfaces interface-specific error messages like
// `"Failed to construct 'Uint8Array': …"`.
macro_rules! typed_array_ctor_wrapper {
    ($fn_name:ident, $ek:expr) => {
        fn $fn_name(
            ctx: &mut NativeContext<'_>,
            this: JsValue,
            args: &[JsValue],
        ) -> Result<JsValue, VmError> {
            construct_typed_array(ctx, this, args, $ek)
        }
    };
}

typed_array_ctor_wrapper!(native_int8_array_ctor, ElementKind::Int8);
typed_array_ctor_wrapper!(native_uint8_array_ctor, ElementKind::Uint8);
typed_array_ctor_wrapper!(native_uint8_clamped_array_ctor, ElementKind::Uint8Clamped);
typed_array_ctor_wrapper!(native_int16_array_ctor, ElementKind::Int16);
typed_array_ctor_wrapper!(native_uint16_array_ctor, ElementKind::Uint16);
typed_array_ctor_wrapper!(native_int32_array_ctor, ElementKind::Int32);
typed_array_ctor_wrapper!(native_uint32_array_ctor, ElementKind::Uint32);
typed_array_ctor_wrapper!(native_float32_array_ctor, ElementKind::Float32);
typed_array_ctor_wrapper!(native_float64_array_ctor, ElementKind::Float64);
typed_array_ctor_wrapper!(native_bigint64_array_ctor, ElementKind::BigInt64);
typed_array_ctor_wrapper!(native_biguint64_array_ctor, ElementKind::BigUint64);

// ---------------------------------------------------------------------------
// Abstract ctor + species/toStringTag getters
// ---------------------------------------------------------------------------

/// Abstract `%TypedArray%` constructor (ES §23.2.1.1).
/// Throws TypeError in BOTH call-mode and new-mode — the spec
/// explicitly forbids direct invocation of the abstract intrinsic
/// (step 2: `"Abstract class TypedArray not directly
/// constructable"`).  Subclasses (Uint8Array, …) are the only
/// allocating entry points.
fn native_abstract_typed_array_ctor(
    _ctx: &mut NativeContext<'_>,
    _this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Err(VmError::type_error(
        "Abstract class TypedArray not directly constructable",
    ))
}

/// `get %TypedArray% [ @@species ]` (ES §23.2.2.4) — returns `this`.
/// Allocating methods (`map` / `filter` / `slice`) that use
/// SpeciesConstructor read this accessor; subclasses inherit it
/// unchanged, so `Uint8Array[Symbol.species] === Uint8Array`.
fn native_typed_array_species_get(
    _ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    Ok(this)
}

/// `%TypedArray%.prototype[@@toStringTag]` getter (ES §23.2.3.32).
/// Reads `[[TypedArrayName]]` — the subclass name string derived
/// from `element_kind`.  Returns `undefined` (NOT throws) if
/// `this` lacks the TypedArray brand, so
/// `Object.prototype.toString.call(plain)` still yields
/// `"[object Object]"` on foreign receivers.
fn native_typed_array_to_tag_get(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let JsValue::Object(id) = this else {
        return Ok(JsValue::Undefined);
    };
    let ObjectKind::TypedArray { element_kind, .. } = ctx.vm.get_object(id).kind else {
        return Ok(JsValue::Undefined);
    };
    Ok(JsValue::String(element_kind_name_sid(
        &ctx.vm.well_known,
        element_kind,
    )))
}

/// Map `ElementKind` to its already-interned subclass-name `StringId`
/// from [`WellKnownStrings`].  Avoids the `strings.intern` round-trip
/// on every `@@toStringTag` read (the getter fires once per
/// `Object.prototype.toString.call(ta)`).
fn element_kind_name_sid(
    wk: &super::super::well_known::WellKnownStrings,
    ek: ElementKind,
) -> StringId {
    match ek {
        ElementKind::Int8 => wk.int8_array_global,
        ElementKind::Uint8 => wk.uint8_array_global,
        ElementKind::Uint8Clamped => wk.uint8_clamped_array_global,
        ElementKind::Int16 => wk.int16_array_global,
        ElementKind::Uint16 => wk.uint16_array_global,
        ElementKind::Int32 => wk.int32_array_global,
        ElementKind::Uint32 => wk.uint32_array_global,
        ElementKind::Float32 => wk.float32_array_global,
        ElementKind::Float64 => wk.float64_array_global,
        ElementKind::BigInt64 => wk.bigint64_array_global,
        ElementKind::BigUint64 => wk.biguint64_array_global,
    }
}

// ---------------------------------------------------------------------------
// Generic prototype accessors
// ---------------------------------------------------------------------------

fn require_typed_array_this(
    ctx: &NativeContext<'_>,
    this: JsValue,
    method: &str,
) -> Result<ObjectId, VmError> {
    let JsValue::Object(id) = this else {
        return Err(VmError::type_error(format!(
            "TypedArray.prototype.{method} called on non-TypedArray"
        )));
    };
    if matches!(ctx.vm.get_object(id).kind, ObjectKind::TypedArray { .. }) {
        Ok(id)
    } else {
        Err(VmError::type_error(format!(
            "TypedArray.prototype.{method} called on non-TypedArray"
        )))
    }
}

fn native_typed_array_get_buffer(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_typed_array_this(ctx, this, "buffer")?;
    let ObjectKind::TypedArray { buffer_id, .. } = ctx.vm.get_object(id).kind else {
        unreachable!("brand-check passed");
    };
    Ok(JsValue::Object(buffer_id))
}

fn native_typed_array_get_byte_offset(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_typed_array_this(ctx, this, "byteOffset")?;
    let ObjectKind::TypedArray { byte_offset, .. } = ctx.vm.get_object(id).kind else {
        unreachable!("brand-check passed");
    };
    Ok(JsValue::Number(f64::from(byte_offset)))
}

fn native_typed_array_get_byte_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_typed_array_this(ctx, this, "byteLength")?;
    let ObjectKind::TypedArray { byte_length, .. } = ctx.vm.get_object(id).kind else {
        unreachable!("brand-check passed");
    };
    Ok(JsValue::Number(f64::from(byte_length)))
}

fn native_typed_array_get_length(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    _args: &[JsValue],
) -> Result<JsValue, VmError> {
    let id = require_typed_array_this(ctx, this, "length")?;
    let ObjectKind::TypedArray {
        byte_length,
        element_kind,
        ..
    } = ctx.vm.get_object(id).kind
    else {
        unreachable!("brand-check passed");
    };
    let bpe = u32::from(element_kind.bytes_per_element());
    let len = byte_length / bpe;
    Ok(JsValue::Number(f64::from(len)))
}

// ---------------------------------------------------------------------------
// Shared constructor dispatch (ES §23.2.5)
// ---------------------------------------------------------------------------

/// Shared body of every TypedArray subclass ctor.  Dispatches on
/// `args[0]` shape per ES §23.2.5:
/// 1. `() / (undefined)` → empty view over fresh zero-byte buffer.
/// 2. `(number)` → `ToIndex(n)`, fresh zero-filled buffer of
///    `n * bpe` bytes (§23.2.5.1.1).
/// 3. `(ArrayBuffer, byteOffset?, length?)` → share buffer bytes,
///    validate alignment (`byteOffset % bpe === 0` — RangeError)
///    and range (§23.2.5.1.3).
/// 4. `(TypedArray)` → fresh buffer of `src.length * dst.bpe` bytes,
///    element-copy with type conversion (§23.2.5.1.2).
/// 5. `(iterable)` where `@@iterator` resolves → consume iterator,
///    allocate buffer, write each element (§23.2.5.1.4).  Any
///    throw during body is closed with `IteratorClose` per
///    §7.4.6 (but a throw from `iter_next` itself is NOT
///    closed — §7.4.7, PR5b R13 lesson).
/// 6. Otherwise → TypeError.
///
/// The pre-allocated receiver carries `new.target.prototype` via
/// `do_new`; we promote its `kind` in-place rather than reassigning
/// `prototype`, so subclasses of our builtins (`class X extends
/// Uint8Array`) inherit correctly (PR5a2 R7.2/R7.3 lesson).
fn construct_typed_array(
    ctx: &mut NativeContext<'_>,
    this: JsValue,
    args: &[JsValue],
    ek: ElementKind,
) -> Result<JsValue, VmError> {
    if !ctx.is_construct() {
        return Err(VmError::type_error(format!(
            "Failed to construct '{}': Please use the 'new' operator",
            ek.name()
        )));
    }
    let JsValue::Object(inst_id) = this else {
        unreachable!("constructor `this` is always an Object after `do_new`");
    };

    let arg0 = args.first().copied().unwrap_or(JsValue::Undefined);
    let arg1 = args.get(1).copied().unwrap_or(JsValue::Undefined);
    let arg2 = args.get(2).copied().unwrap_or(JsValue::Undefined);

    let (buffer_id, byte_offset, byte_length) = match arg0 {
        JsValue::Undefined => allocate_fresh_buffer(ctx, 0)?,
        JsValue::Object(src_id) => match ctx.vm.get_object(src_id).kind {
            ObjectKind::ArrayBuffer => init_from_array_buffer(ctx, src_id, arg1, arg2, ek)?,
            ObjectKind::TypedArray { .. } => init_from_typed_array(ctx, src_id, ek)?,
            _ => init_from_iterable(ctx, arg0, ek)?,
        },
        // Plain number (or anything number-coercible via ToNumber)
        // → length form.  Strings like `"5"` coerce too; this
        // matches V8 / SpiderMonkey.  NaN → 0-length per ToIndex.
        _ => {
            let length = coerce::to_index_u32(ctx, arg0, ek.name(), "length")?;
            let byte_len = length
                .checked_mul(u32::from(ek.bytes_per_element()))
                .ok_or_else(|| {
                    VmError::range_error(format!(
                        "Failed to construct '{}': length too large",
                        ek.name()
                    ))
                })?;
            allocate_fresh_buffer(ctx, byte_len)?
        }
    };

    // Preserve `prototype` on the pre-allocated instance so
    // `new.target.prototype` chains work for subclasses (PR5a2 R7.2/R7.3).
    ctx.vm.get_object_mut(inst_id).kind = ObjectKind::TypedArray {
        buffer_id,
        byte_offset,
        byte_length,
        element_kind: ek,
    };
    Ok(JsValue::Object(inst_id))
}

/// Allocate a fresh `ArrayBuffer` of `byte_len` zero bytes and
/// return its `(ObjectId, byte_offset=0, byte_length)` triple.
/// Uses the shared `body_data` store so GC sweep prunes it
/// alongside other ArrayBuffers.
pub(super) fn allocate_fresh_buffer(
    ctx: &mut NativeContext<'_>,
    byte_len: u32,
) -> Result<(ObjectId, u32, u32), VmError> {
    let bytes: Arc<[u8]> = if byte_len == 0 {
        Arc::from(&[][..])
    } else {
        vec![0_u8; byte_len as usize].into()
    };
    let buf_id = super::array_buffer::create_array_buffer_from_bytes(ctx.vm, bytes);
    Ok((buf_id, 0, byte_len))
}

// ---------------------------------------------------------------------------
// Low-level element read/write (shared with indexed access in C3)
// ---------------------------------------------------------------------------

/// Read the element at `index` from the buffer backing a
/// TypedArray, decoded per `ek`.  Little-endian byte order
/// (elidex convention, documented at module header).  Missing
/// body_data entry (e.g. freshly allocated zero-byte buffer) is
/// treated as all zeros.
///
/// Takes `&mut VmInner` because BigInt element reads need to
/// `alloc` a BigIntId on the interning pool.  For non-BigInt
/// subclasses no allocation occurs.
pub(crate) fn read_element_raw(
    vm: &mut VmInner,
    buffer_id: ObjectId,
    byte_offset: u32,
    index: u32,
    ek: ElementKind,
) -> JsValue {
    let bpe = u32::from(ek.bytes_per_element());
    let abs = (byte_offset + index * bpe) as usize;
    // Snapshot exactly `bpe` bytes per element kind so each
    // subscript copies only the bytes the decoder will actually
    // consume — small-element reads (e.g. `Uint8Array`) are hot
    // and must not pay for the wider fixed-size scratch.  The
    // const-generic `read_into` produces a per-arm fixed-size
    // array; the BigInt arms own a `&mut vm` reborrow afterwards
    // so the snapshot must complete before the `bigints.alloc`.
    match ek {
        ElementKind::Int8 => {
            let s = super::byte_io::read_into::<1>(&vm.body_data, buffer_id, abs);
            JsValue::Number(f64::from(s[0] as i8))
        }
        ElementKind::Uint8 | ElementKind::Uint8Clamped => {
            let s = super::byte_io::read_into::<1>(&vm.body_data, buffer_id, abs);
            JsValue::Number(f64::from(s[0]))
        }
        ElementKind::Int16 => {
            let s = super::byte_io::read_into::<2>(&vm.body_data, buffer_id, abs);
            JsValue::Number(f64::from(i16::from_le_bytes(s)))
        }
        ElementKind::Uint16 => {
            let s = super::byte_io::read_into::<2>(&vm.body_data, buffer_id, abs);
            JsValue::Number(f64::from(u16::from_le_bytes(s)))
        }
        ElementKind::Int32 => {
            let s = super::byte_io::read_into::<4>(&vm.body_data, buffer_id, abs);
            JsValue::Number(f64::from(i32::from_le_bytes(s)))
        }
        ElementKind::Uint32 => {
            let s = super::byte_io::read_into::<4>(&vm.body_data, buffer_id, abs);
            JsValue::Number(f64::from(u32::from_le_bytes(s)))
        }
        ElementKind::Float32 => {
            let s = super::byte_io::read_into::<4>(&vm.body_data, buffer_id, abs);
            JsValue::Number(f64::from(f32::from_le_bytes(s)))
        }
        ElementKind::Float64 => {
            let s = super::byte_io::read_into::<8>(&vm.body_data, buffer_id, abs);
            JsValue::Number(f64::from_le_bytes(s))
        }
        ElementKind::BigInt64 => {
            let s = super::byte_io::read_into::<8>(&vm.body_data, buffer_id, abs);
            let v = i64::from_le_bytes(s);
            let bi = num_bigint::BigInt::from(v);
            JsValue::BigInt(vm.bigints.alloc(bi))
        }
        ElementKind::BigUint64 => {
            let s = super::byte_io::read_into::<8>(&vm.body_data, buffer_id, abs);
            let v = u64::from_le_bytes(s);
            let bi = num_bigint::BigInt::from(v);
            JsValue::BigInt(vm.bigints.alloc(bi))
        }
    }
}

/// Coerce `value` per `ek` and serialise the per-element
/// little-endian byte sequence into `out`, returning the number of
/// bytes written (always equal to `ek.bytes_per_element()`).
/// Shared by [`write_element_raw`] (single-element writes) and
/// [`super::typed_array_methods::native_typed_array_fill`] (one
/// coerce per fill, not per element).
///
/// Coercion may run user code (`valueOf` / `Symbol.toPrimitive`)
/// and throw — callers therefore invoke this before any
/// irreversible mutation, so a thrown coercion leaves the backing
/// buffer untouched.
pub(crate) fn coerce_element_to_le_bytes(
    ctx: &mut NativeContext<'_>,
    ek: ElementKind,
    value: JsValue,
    out: &mut [u8; 8],
) -> Result<usize, VmError> {
    Ok(match ek {
        ElementKind::Int8 => {
            let v = super::super::coerce::to_int8(ctx.vm, value)?;
            out[0] = v as u8;
            1
        }
        ElementKind::Uint8 => {
            let v = super::super::coerce::to_uint8(ctx.vm, value)?;
            out[0] = v;
            1
        }
        ElementKind::Uint8Clamped => {
            let v = super::super::coerce::to_uint8_clamp(ctx.vm, value)?;
            out[0] = v;
            1
        }
        ElementKind::Int16 => {
            let v = super::super::coerce::to_int16(ctx.vm, value)?;
            out[..2].copy_from_slice(&v.to_le_bytes());
            2
        }
        ElementKind::Uint16 => {
            let v = super::super::coerce::to_uint16(ctx.vm, value)?;
            out[..2].copy_from_slice(&v.to_le_bytes());
            2
        }
        ElementKind::Int32 => {
            let v = super::super::coerce::to_int32(ctx.vm, value)?;
            out[..4].copy_from_slice(&v.to_le_bytes());
            4
        }
        ElementKind::Uint32 => {
            let v = super::super::coerce::to_uint32(ctx.vm, value)?;
            out[..4].copy_from_slice(&v.to_le_bytes());
            4
        }
        ElementKind::Float32 => {
            let n = super::super::coerce::to_number(ctx.vm, value)?;
            #[allow(clippy::cast_possible_truncation)]
            let v = n as f32;
            out[..4].copy_from_slice(&v.to_le_bytes());
            4
        }
        ElementKind::Float64 => {
            let n = super::super::coerce::to_number(ctx.vm, value)?;
            out[..8].copy_from_slice(&n.to_le_bytes());
            8
        }
        ElementKind::BigInt64 => {
            let v = super::super::natives_bigint::to_bigint64(ctx, value)?;
            out[..8].copy_from_slice(&v.to_le_bytes());
            8
        }
        ElementKind::BigUint64 => {
            let v = super::super::natives_bigint::to_biguint64(ctx, value)?;
            out[..8].copy_from_slice(&v.to_le_bytes());
            8
        }
    })
    .map(|len| {
        debug_assert_eq!(
            len,
            usize::from(ek.bytes_per_element()),
            "coerce_element_to_le_bytes wrote {len} bytes but ek={ek:?} declares bytes_per_element()={}",
            ek.bytes_per_element()
        );
        len
    })
}

/// Write `value` into the buffer at `index`, coerced per `ek`.
/// Returns `Err` when BigInt coercion fails (writing a Number into
/// a `BigInt64Array` — `ToBigInt` rejects) or when user-level
/// coercion (valueOf / Symbol.toPrimitive) throws.
///
/// The backing `Arc<[u8]>` is treated as immutable: every write
/// produces a fresh `Arc::from(Vec)` that replaces the map entry.
/// This keeps the spec-mandated "views share bytes" invariant —
/// every view reads through `body_data[buffer_id]`, which always
/// holds the latest snapshot.  The O(N)-per-write cost is
/// acceptable for C2; a byte-level interior-mutability refactor
/// lands with PR-spec-polish SP9.
pub(crate) fn write_element_raw(
    ctx: &mut NativeContext<'_>,
    buffer_id: ObjectId,
    byte_offset: u32,
    index: u32,
    ek: ElementKind,
    value: JsValue,
) -> Result<(), VmError> {
    let bpe = u32::from(ek.bytes_per_element());
    let abs = (byte_offset + index * bpe) as usize;

    // Coerce first — user code may run (valueOf / toPrimitive)
    // and throw.  Scratch buffer holds the encoded little-endian
    // bytes; the actual write (which replaces the Arc entry)
    // only runs if coercion succeeds.
    let mut scratch = [0_u8; 8];
    let written_len = coerce_element_to_le_bytes(ctx, ek, value, &mut scratch)?;

    // Clone the existing buffer, grow if needed, apply the element
    // write, install the fresh `Arc<[u8]>` so other views over the
    // same `buffer_id` see the new bytes on their next access.
    super::byte_io::write_at(
        &mut ctx.vm.body_data,
        buffer_id,
        abs,
        &scratch[..written_len],
    );
    Ok(())
}
