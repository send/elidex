//! `%TypedArray%` registration + `%TypedArray%.prototype` install
//! pipeline.  Sibling-extracted from [`super::typed_array`] to keep
//! that file under the project's 1000-line convention as the
//! prototype-method install table grew through SP8b/SP8c-A/SP8c-B
//! (filter / map / reduce / reduceRight / sort / flatMap +
//! toLocaleString — 28 entries at SP8c-B).
//!
//! Hosts the registration `impl VmInner` methods, the per-subclass
//! dispatch table, and the abstract-ctor + species / toStringTag
//! accessors — anything that runs only at `Vm::new` and never
//! again on the hot path.  Element read/write primitives
//! ([`super::typed_array::read_element_raw`] /
//! [`super::typed_array::write_element_raw`] /
//! [`super::typed_array::coerce_element_to_le_bytes`]) and the
//! generic IDL accessors stay in [`super::typed_array`] alongside
//! the receiver brand-check helper, since those are reached every
//! TypedArray indexed access.

#![cfg(feature = "engine")]

use super::super::coerce;
use super::super::shape::{self, PropertyAttrs};
use super::super::value::{
    ElementKind, JsValue, NativeContext, Object, ObjectId, ObjectKind, PropertyKey,
    PropertyStorage, PropertyValue, StringId, VmError,
};
use super::super::{NativeFn, VmInner};

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

        // Static `%TypedArray%.of` / `.from` (ES §23.2.2.{1,2}) —
        // install body lives in [`super::typed_array_static`]
        // alongside the natives' impls + the `subclass_array_ctors`
        // dispatch helper.
        super::typed_array_static::install_typed_array_static_methods(self, abstract_ctor);
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
                super::typed_array::native_typed_array_get_buffer as NativeFn,
            ),
            (
                self.well_known.byte_offset,
                super::typed_array::native_typed_array_get_byte_offset as NativeFn,
            ),
            (
                self.well_known.byte_length,
                super::typed_array::native_typed_array_get_byte_length as NativeFn,
            ),
            (
                self.well_known.length,
                super::typed_array::native_typed_array_get_length as NativeFn,
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

        // Core method suite.  See `typed_array_methods` /
        // `typed_array_hof` for the spec-aligned bodies; each
        // method is installed on `%TypedArray%.prototype` (shared
        // across all 11 subclasses via prototype chain).  Names
        // are pre-interned in `WellKnownStrings` so this table
        // doesn't pay the per-call `strings.intern(...)`
        // round-trip during `Vm::new`.
        let methods: [(StringId, NativeFn); 28] = [
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
                super::typed_array_hof::native_typed_array_for_each as NativeFn,
            ),
            (
                self.well_known.every,
                super::typed_array_hof::native_typed_array_every as NativeFn,
            ),
            (
                self.well_known.some,
                super::typed_array_hof::native_typed_array_some as NativeFn,
            ),
            (
                self.well_known.find,
                super::typed_array_hof::native_typed_array_find as NativeFn,
            ),
            (
                self.well_known.find_index,
                super::typed_array_hof::native_typed_array_find_index as NativeFn,
            ),
            (
                self.well_known.find_last,
                super::typed_array_hof::native_typed_array_find_last as NativeFn,
            ),
            (
                self.well_known.find_last_index,
                super::typed_array_hof::native_typed_array_find_last_index as NativeFn,
            ),
            (
                self.well_known.map,
                super::typed_array_hof::native_typed_array_map as NativeFn,
            ),
            (
                self.well_known.filter,
                super::typed_array_hof::native_typed_array_filter as NativeFn,
            ),
            (
                self.well_known.reduce,
                super::typed_array_hof::native_typed_array_reduce as NativeFn,
            ),
            (
                self.well_known.reduce_right,
                super::typed_array_hof::native_typed_array_reduce_right as NativeFn,
            ),
            (
                self.well_known.sort,
                super::typed_array_hof::native_typed_array_sort as NativeFn,
            ),
            (
                self.well_known.flat_map,
                super::typed_array_hof::native_typed_array_flat_map as NativeFn,
            ),
            (
                self.well_known.to_locale_string,
                super::typed_array_methods::native_typed_array_to_locale_string as NativeFn,
            ),
        ];
        for (name_sid, fn_ptr) in methods {
            self.install_native_method(proto_id, name_sid, fn_ptr, PropertyAttrs::METHOD);
        }

        // `%TypedArray%.prototype[Symbol.iterator]` — spec-mandated
        // identity-equal to `.values` (ES §23.2.3.33).  Install
        // after `values` so we can reuse the same function id.
        let values_slot =
            coerce::get_property(self, proto_id, PropertyKey::String(self.well_known.values));
        if let Some(coerce::PropertyResult::Data(JsValue::Object(values_fn))) = values_slot {
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
            if let Some(coerce::PropertyResult::Data(JsValue::Object(array_to_string))) =
                coerce::get_property(self, array_proto, PropertyKey::String(to_string_sid))
            {
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
    /// [`super::typed_array::construct_typed_array`] impl
    /// parameterised by [`ElementKind`].
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

        // Reverse-lookup table for the static `%TypedArray%.of` /
        // `.from` natives — see `subclass_array_ctors` doc on `VmInner`.
        self.subclass_array_ctors[entry.element_kind.index()] = Some(ctor);
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
            super::typed_array::construct_typed_array(ctx, this, args, $ek)
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
/// from [`super::super::well_known::WellKnownStrings`].  Avoids the
/// `strings.intern` round-trip on every `@@toStringTag` read (the
/// getter fires once per `Object.prototype.toString.call(ta)`).
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
